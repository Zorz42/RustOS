use core::arch::asm;
use core::hint::black_box;
use core::mem::size_of;
use core::ptr::{addr_of, write_bytes};
use core::sync::atomic::{fence, Ordering};
use crate::spinlock::Lock;
use crate::virtio::definitions::{virtio_reg_read, VirtqAvail, VirtqDesc, VirtqUsed, MmioOffset, NUM, VIRTIO_CONFIG_S_ACKNOWLEDGE, VIRTIO_CONFIG_S_DRIVER, VIRTIO_F_ANY_LAYOUT, VIRTIO_RING_F_EVENT_IDX, VIRTIO_RING_F_INDIRECT_DESC, VIRTIO_CONFIG_S_FEATURES_OK, VIRTIO_CONFIG_S_DRIVER_OK, VRING_DESC_F_NEXT, VRING_DESC_F_WRITE, MAX_VIRTIO_ID, virtio_reg_write, VIRTIO_MAGIC};
use std::Vec;
use crate::memory::{alloc_page, virt_to_phys, VirtAddr, PAGE_SIZE};
use crate::riscv::get_core_id;

pub const VIRTIO_BLK_F_RO: u32 = 5; // Disk is read-only
pub const VIRTIO_BLK_F_SCSI: u32 = 7; // Supports scsi command passthru
pub const VIRTIO_BLK_F_CONFIG_WCE: u32 = 11; // Writeback mode available in config
pub const VIRTIO_BLK_F_MQ: u32 = 12; // support more than one vq

// these are specific to virtio block devices, e.g. disks,
// described in Section 5.2 of the spec.

pub const VIRTIO_BLK_T_IN: u32 = 0; // read the disk
pub const VIRTIO_BLK_T_OUT: u32 = 1; // write the disk

// the format of the first descriptor in a disk request.
// to be followed by two more descriptors containing
// the block, and a one-byte status.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct VirtioBlqReq {
    pub typ: u32, // VIRTIO_BLK_T_IN or ..._OUT
    pub reserved: u32,
    pub sector: u64,
}

struct Buf {
    data: [u8; 512],
}

pub struct Disk {
    // a set (not a ring) of DMA descriptors, with which the
    // driver tells the device where to read and write individual
    // disk operations. there are NUM descriptors.
    // most commands consist of a "chain" (a linked list) of a couple of
    // these descriptors.
    desc: *mut VirtqDesc,

    // a ring in which the driver writes descriptor numbers
    // that the driver would like the device to process.  it only
    // includes the head descriptor of each chain. the ring has
    // NUM elements.
    avail: *mut VirtqAvail,

    // a ring in which the device writes descriptor numbers that
    // the device has finished processing (just the head of each chain).
    // there are NUM used ring entries.
    used: *mut VirtqUsed,

    // our own book-keeping.
    free: [bool; NUM], // is a descriptor free?
    used_idx: u16,     // we've looked this far in used[2..NUM].

    // track info about in-flight operations,
    // for use when completion interrupt arrives.
    // indexed by first descriptor index of chain.
    info: [bool; NUM],

    vdisk_lock: Lock,

    id: u64, // virtio device id

    size: usize, // size of the disk

    irq_waiting: bool, // if an irq was cancelled because it was locked. do it when it unlocks
}

pub fn get_disk_at(id: u64) -> Option<&'static mut Disk> {
    if virtio_reg_read(id, MmioOffset::MagicValue) != VIRTIO_MAGIC
        || virtio_reg_read(id, MmioOffset::Version) != 2
        || virtio_reg_read(id, MmioOffset::DeviceId) != 2
        || virtio_reg_read(id, MmioOffset::VendorId) != 0x554d4551
    {
        return None;
    }

    let disk = unsafe {&mut *(alloc_page() as *mut Disk) };

    *disk = Disk {
        desc: 0 as *mut VirtqDesc,
        avail: 0 as *mut VirtqAvail,
        used: 0 as *mut VirtqUsed,
        free: [true; NUM],
        used_idx: 0,
        info: [false; NUM],
        vdisk_lock: Lock::new(),
        id,
        size: 0,
        irq_waiting: false,
    };

    unsafe {
        DISKS[id as usize] = disk as *mut Disk;
    }

    let mut status = 0;

    status |= VIRTIO_CONFIG_S_ACKNOWLEDGE;

    virtio_reg_write(id, MmioOffset::Status, status);

    status |= VIRTIO_CONFIG_S_DRIVER;
    virtio_reg_write(id, MmioOffset::Status, status);

    // negotiate features
    let mut features = virtio_reg_read(id, MmioOffset::DeviceFeatures);
    features &= !(1 << VIRTIO_BLK_F_RO);
    features &= !(1 << VIRTIO_BLK_F_SCSI);
    features &= !(1 << VIRTIO_BLK_F_CONFIG_WCE);
    features &= !(1 << VIRTIO_BLK_F_MQ);
    features &= !(1 << VIRTIO_F_ANY_LAYOUT);
    features &= !(1 << VIRTIO_RING_F_EVENT_IDX);
    features &= !(1 << VIRTIO_RING_F_INDIRECT_DESC);
    virtio_reg_write(id, MmioOffset::DriverFeatures, features);

    status |= VIRTIO_CONFIG_S_FEATURES_OK;
    virtio_reg_write(id, MmioOffset::Status, status);

    // reread and check
    status = virtio_reg_read(id, MmioOffset::Status);
    assert_eq!(status & VIRTIO_CONFIG_S_FEATURES_OK, VIRTIO_CONFIG_S_FEATURES_OK);

    virtio_reg_write(id, MmioOffset::QueueSel, 0);

    assert_eq!(virtio_reg_read(id, MmioOffset::QueueReady), 0);

    assert!(virtio_reg_read(id, MmioOffset::QueueNumMax) >= NUM as u32);

    disk.desc = alloc_page() as *mut VirtqDesc;
    disk.avail = alloc_page() as *mut VirtqAvail;
    disk.used = alloc_page() as *mut VirtqUsed;

    unsafe {
        write_bytes(disk.desc as *mut u8, 0, PAGE_SIZE as usize);
        write_bytes(disk.avail as *mut u8, 0, PAGE_SIZE as usize);
        write_bytes(disk.used as *mut u8, 0, PAGE_SIZE as usize);
    }

    virtio_reg_write(id, MmioOffset::QueueNum, NUM as u32);

    virtio_reg_write(id, MmioOffset::QueueDescLow, (disk.desc as u64 & 0xFFFFFFFF) as u32);
    virtio_reg_write(id, MmioOffset::QueueDescHigh, ((disk.desc as u64 >> 32) & 0xFFFFFFFF) as u32);
    virtio_reg_write(id, MmioOffset::DriverDescLow, (disk.avail as u64 & 0xFFFFFFFF) as u32);
    virtio_reg_write(id, MmioOffset::DriverDescHigh, ((disk.avail as u64 >> 32) & 0xFFFFFFFF) as u32);
    virtio_reg_write(id, MmioOffset::DeviceDescLow, (disk.used as u64 & 0xFFFFFFFF) as u32);
    virtio_reg_write(id, MmioOffset::DeviceDescHigh, ((disk.used as u64 >> 32) & 0xFFFFFFFF) as u32);

    // queue is ready
    virtio_reg_write(id, MmioOffset::QueueReady, 1);

    // we are completely ready
    status |= VIRTIO_CONFIG_S_DRIVER_OK;
    virtio_reg_write(id, MmioOffset::Status, status);

    // get the disk size
    disk.size = virtio_reg_read(id, MmioOffset::Config) as usize;

    Some(disk)
}

impl Disk {
    fn alloc_desc(&mut self) -> Option<usize> {
        for i in 0..NUM {
            if self.free[i] {
                self.free[i] = false;
                return Some(i);
            }
        }
        None
    }

    fn get_desc(&mut self, idx: usize) -> &mut VirtqDesc {
        assert!(idx < NUM);
        unsafe { &mut *self.desc.add(idx) }
    }

    fn free_desc(&mut self, idx: usize) {
        assert!(!self.free[idx]);

        let desc = self.get_desc(idx);
        desc.next = 0;
        desc.flags = 0;
        desc.next = 0;
        desc.len = 0;
        self.free[idx] = true;
    }

    fn alloc_3desc(&mut self) -> Option<[usize; 3]> {
        let mut res = [0; 3];

        for (i, r) in res.iter_mut().enumerate() {
            let desc = self.alloc_desc();
            if let Some(desc) = desc {
                *r = desc;
            } else {
                for j in 0..i {
                    self.free_desc(j);
                }
                return None;
            }
        }

        Some(res)
    }

    fn free_chain(&mut self, mut idx: usize) {
        loop {
            let flags = self.get_desc(idx).flags;
            let next = self.get_desc(idx).next as usize;
            self.free_desc(idx);
            if (flags & VRING_DESC_F_NEXT) == 0 {
                break;
            }
            idx = next;
        }
    }

    fn virtio_disk_rw(&mut self, data: &[u8; 512], sector: u32, write: bool) {
        self.vdisk_lock.spinlock();

        let idx = self.alloc_3desc().unwrap();

        let mut buf0 = VirtioBlqReq {
            typ: if write { VIRTIO_BLK_T_OUT } else { VIRTIO_BLK_T_IN },
            reserved: 0,
            sector: sector as u64,
        };

        let desc0 = self.get_desc(idx[0]);
        desc0.addr = addr_of!(buf0) as u64;
        desc0.len = size_of::<VirtioBlqReq>() as u32;
        desc0.flags = VRING_DESC_F_NEXT;
        desc0.next = idx[1] as u16;

        let desc1 = self.get_desc(idx[1]);
        desc1.addr = virt_to_phys(data.as_ptr() as VirtAddr).unwrap();
        desc1.len = 512;
        desc1.flags = if write {0} else {VRING_DESC_F_WRITE};
        desc1.flags |= VRING_DESC_F_NEXT;
        desc1.next = idx[2] as u16;

        let mut status = 0xFF;

        let addr = addr_of!(status) as u64;
        let desc2 = self.get_desc(idx[2]);
        desc2.addr = addr;
        desc2.len = 1;
        desc2.flags = VRING_DESC_F_WRITE;
        desc2.next = 0;

        status = black_box(status); // because virtio does stuff to it
        buf0 = black_box(buf0);

        self.info[idx[0]] = true;

        unsafe {
            let idx2 = (*self.avail).idx as usize;
            (*self.avail).ring[idx2 % NUM] = idx[0] as u16;
        }

        fence(Ordering::Release);

        unsafe {
            (*self.avail).idx += 1;
        }

        fence(Ordering::Release);

        virtio_reg_write(self.id, MmioOffset::QueueNotify, 0);

        self.vdisk_lock.unlock();
        if self.irq_waiting {
            disk_irq(self.id as u32 + 1);
        }

        while self.info[idx[0]] {
            unsafe {
                asm!("wfi");
            }
            self.info = black_box(self.info); // interrupt might change it
        }

        self.vdisk_lock.spinlock();

        assert_eq!(status, 0);
        self.info[idx[0]] = false;
        self.free_chain(idx[0]);

        self.vdisk_lock.unlock();

        if self.irq_waiting {
            disk_irq(self.id as u32 + 1);
        }

        black_box(data); // we need that data all the way through
    }

    pub fn read(&mut self, sector: usize) -> [u8; 512] {
        assert!(sector < self.size);

        let data = [0; 512];
        self.virtio_disk_rw(&data, sector as u32, false);
        black_box(data) // because data is being written to
    }

    pub fn write(&mut self, sector: usize, data: &[u8; 512]) {
        assert!(sector < self.size);

        self.virtio_disk_rw(data, sector as u32, true);

    }

    pub const fn size(&self) -> usize {
        self.size
    }
}

pub fn scan_for_disks() -> Vec<&'static mut Disk> {
    let mut vec = Vec::new();
    for id in 0..MAX_VIRTIO_ID {
        if let Some(disk) = get_disk_at(id) {
            vec.push(disk);
        }
    }

    vec
}

static mut DISKS: [*mut Disk; MAX_VIRTIO_ID as usize] = [0 as *mut Disk; MAX_VIRTIO_ID as usize];

pub fn disk_irq(irq: u32) {
    if irq == 0 || irq > MAX_VIRTIO_ID as u32 {
        return;
    }
    let id = irq as u64 - 1;

    let disk_ptr = unsafe { DISKS[id as usize] };
    if disk_ptr == 0 as *mut Disk {
        return;
    }

    let disk = unsafe { &mut *disk_ptr };

    if disk.vdisk_lock.locked_by() == get_core_id() as i32 {
        disk.irq_waiting = true;
        return;
    }
    disk.vdisk_lock.spinlock();

    virtio_reg_write(id, MmioOffset::InterruptAck, virtio_reg_read(id, MmioOffset::InterruptStatus) & 0x3);

    fence(Ordering::Release);

    let used = unsafe { &mut *disk.used };

    while disk.used_idx != used.idx {
        fence(Ordering::Release);
        let id = used.ring[disk.used_idx as usize % NUM].id;

        unsafe {
            disk.info[id as usize] = false;
        }

        disk.used_idx += 1;
    }

    disk.vdisk_lock.unlock();
}
