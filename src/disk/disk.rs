use core::mem::size_of;
use core::ptr::{addr_of, write_bytes};
use core::sync::atomic::{fence, Ordering};
use crate::spinlock::Lock;
use crate::virtio::{virtio_reg, VirtioBlqReq, VirtqAvail, VirtqDesc, VirtqUsed, MAX_VIRTIO_ID, NUM, VIRTIO_BLK_F_CONFIG_WCE, VIRTIO_BLK_F_MQ, VIRTIO_BLK_F_RO, VIRTIO_BLK_F_SCSI, VIRTIO_CONFIG_S_ACKNOWLEDGE, VIRTIO_CONFIG_S_DRIVER, VIRTIO_F_ANY_LAYOUT, VIRTIO_MMIO_DEVICE_FEATURES, VIRTIO_MMIO_DEVICE_ID, VIRTIO_MMIO_DRIVER_FEATURES, VIRTIO_MMIO_MAGIC_VALUE, VIRTIO_MMIO_STATUS, VIRTIO_MMIO_VENDOR_ID, VIRTIO_MMIO_VERSION, VIRTIO_RING_F_EVENT_IDX, VIRTIO_RING_F_INDIRECT_DESC, VIRTIO_CONFIG_S_FEATURES_OK, VIRTIO_MMIO_QUEUE_SEL, VIRTIO_MMIO_QUEUE_READY, VIRTIO_MMIO_QUEUE_NUM_MAX, VIRTIO_MMIO_QUEUE_NUM, VIRTIO_MMIO_DEVICE_DESC_LOW, VIRTIO_MMIO_DEVICE_DESC_HIGH, VIRTIO_MMIO_QUEUE_DESC_LOW, VIRTIO_MMIO_QUEUE_DESC_HIGH, VIRTIO_MMIO_DRIVER_DESC_LOW, VIRTIO_MMIO_DRIVER_DESC_HIGH, VIRTIO_CONFIG_S_DRIVER_OK, VRING_DESC_F_NEXT, VIRTIO_BLK_T_OUT, VIRTIO_BLK_T_IN, VRING_DESC_F_WRITE, VIRTIO_MMIO_QUEUE_NOTIFY, VIRTIO_MMIO_INTERRUPT_ACK, VIRTIO_MMIO_INTERRUPT_STATUS, VIRTIO_MMIO_CONFIG};
use std::{print, println, Vec};
use crate::memory::{alloc_page, PAGE_SIZE};

struct Buf {
    valid: i32, // has data been read from disk?
    disk: i32,  // does disk "own" buf?
    dev: u32,
    sector: u32,
    lock: Lock,
    refcnt: u32,
    prev: *mut Buf, // LRU cache list
    next: *mut Buf,
    data: [u8; 512],
}

#[derive(Clone, Copy)]
struct Info {
    b: *mut Buf,
    status: u8,
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
    info: [Info; NUM],

    // disk command headers.
    // one-for-one with descriptors, for convenience.
    ops: [VirtioBlqReq; NUM],

    vdisk_lock: Lock,

    id: u64,

    size: usize,
}

pub fn get_disk_at(id: u64) -> Option<&'static mut Disk> {
    if (*virtio_reg(id, VIRTIO_MMIO_MAGIC_VALUE) != 0x74726976
        || *virtio_reg(id, VIRTIO_MMIO_VERSION) != 2
        || *virtio_reg(id, VIRTIO_MMIO_DEVICE_ID) != 2
        || *virtio_reg(id, VIRTIO_MMIO_VENDOR_ID) != 0x554d4551)
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
        info: [Info { b: 0 as *mut Buf, status: 0 }; NUM],
        ops: [VirtioBlqReq { typ: 0, reserved: 0, sector: 0 }; NUM],
        vdisk_lock: Lock::new(),
        id,
        size: 0,
    };

    unsafe {
        DISKS[id as usize] = disk as *mut Disk;
    }

    let mut status = 0;

    status |= VIRTIO_CONFIG_S_ACKNOWLEDGE;
    *virtio_reg(id, VIRTIO_MMIO_STATUS) = status;

    status |= VIRTIO_CONFIG_S_DRIVER;
    *virtio_reg(id, VIRTIO_MMIO_STATUS) = status;

    // negotiate features
    let mut features = *virtio_reg(id, VIRTIO_MMIO_DEVICE_FEATURES);
    features &= !(1 << VIRTIO_BLK_F_RO);
    features &= !(1 << VIRTIO_BLK_F_SCSI);
    features &= !(1 << VIRTIO_BLK_F_CONFIG_WCE);
    features &= !(1 << VIRTIO_BLK_F_MQ);
    features &= !(1 << VIRTIO_F_ANY_LAYOUT);
    features &= !(1 << VIRTIO_RING_F_EVENT_IDX);
    features &= !(1 << VIRTIO_RING_F_INDIRECT_DESC);
    *virtio_reg(id, VIRTIO_MMIO_DRIVER_FEATURES) = features;

    status |= VIRTIO_CONFIG_S_FEATURES_OK;
    *virtio_reg(id, VIRTIO_MMIO_STATUS) = status;


    // reread and check
    status = *virtio_reg(id, VIRTIO_MMIO_STATUS);
    assert_eq!(status & VIRTIO_CONFIG_S_FEATURES_OK, VIRTIO_CONFIG_S_FEATURES_OK);

    *virtio_reg(id, VIRTIO_MMIO_QUEUE_SEL) = 0;

    assert_eq!(*virtio_reg(id, VIRTIO_MMIO_QUEUE_READY), 0);

    assert!(*virtio_reg(id, VIRTIO_MMIO_QUEUE_NUM_MAX) >= NUM as u32);

    disk.desc = alloc_page() as *mut VirtqDesc;
    disk.avail = alloc_page() as *mut VirtqAvail;
    disk.used = alloc_page() as *mut VirtqUsed;

    unsafe {
        write_bytes(disk.desc as *mut u8, 0, PAGE_SIZE as usize);
        write_bytes(disk.avail as *mut u8, 0, PAGE_SIZE as usize);
        write_bytes(disk.used as *mut u8, 0, PAGE_SIZE as usize);
    }

    *virtio_reg(id, VIRTIO_MMIO_QUEUE_NUM) = NUM as u32;

    *virtio_reg(id, VIRTIO_MMIO_QUEUE_DESC_LOW) = (disk.desc as u64 & 0xFFFFFFFF) as u32;
    *virtio_reg(id, VIRTIO_MMIO_QUEUE_DESC_HIGH) = ((disk.desc as u64 >> 32) & 0xFFFFFFFF) as u32;
    *virtio_reg(id, VIRTIO_MMIO_DRIVER_DESC_LOW) = (disk.avail as u64 & 0xFFFFFFFF) as u32;
    *virtio_reg(id, VIRTIO_MMIO_DRIVER_DESC_HIGH) = ((disk.avail as u64 >> 32) & 0xFFFFFFFF) as u32;
    *virtio_reg(id, VIRTIO_MMIO_DEVICE_DESC_LOW) = (disk.used as u64 & 0xFFFFFFFF) as u32;
    *virtio_reg(id, VIRTIO_MMIO_DEVICE_DESC_HIGH) = ((disk.used as u64 >> 32) & 0xFFFFFFFF) as u32;

    // queue is ready
    *virtio_reg(id, VIRTIO_MMIO_QUEUE_READY) = 1;

    // we are completely ready
    status |= VIRTIO_CONFIG_S_DRIVER_OK;
    *virtio_reg(id, VIRTIO_MMIO_STATUS) = status;

    // get the disk size
    disk.size = *virtio_reg(id, VIRTIO_MMIO_CONFIG) as usize;

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

    fn get_desc(&self, idx: usize) -> &mut VirtqDesc {
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

        for i in 0..3 {
            let desc = self.alloc_desc();
            if let Some(desc) = desc {
                res[i] = desc;
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
            } else {
                idx = next;
            }
        }
    }

    fn virtio_disk_rw(&mut self, buf: &mut Buf, write: bool) {
        let idx = self.alloc_3desc().unwrap();

        let buf0 = unsafe { &mut *((&mut self.ops[idx[0]]) as *mut VirtioBlqReq) };
        buf0.typ = if write { VIRTIO_BLK_T_OUT } else { VIRTIO_BLK_T_IN };
        buf0.reserved = 0;
        buf0.sector = buf.sector as u64;

        let desc0 = self.get_desc(idx[0]);
        desc0.addr = addr_of!(*buf0) as u64;
        desc0.len = size_of::<VirtioBlqReq>() as u32;
        desc0.flags = VRING_DESC_F_NEXT;
        desc0.next = idx[1] as u16;

        let desc1 = self.get_desc(idx[1]);
        desc1.addr = buf.data.as_ptr() as u64;
        desc1.len = 512;
        desc1.flags = if write {0} else {VRING_DESC_F_WRITE};
        desc1.flags |= VRING_DESC_F_NEXT;
        desc1.next = idx[2] as u16;

        self.info[idx[0]].status = 0xFF;

        let desc2 = self.get_desc(idx[2]);
        desc2.addr = addr_of!(self.info[idx[0]].status) as u64;
        desc2.len = 1;
        desc2.flags = VRING_DESC_F_WRITE;
        desc2.next = 0;

        buf.disk = 1;
        self.info[idx[0]].b = buf;

        unsafe {
            let idx2 = (*self.avail).idx as usize;
            (*self.avail).ring[idx2 % NUM] = idx[0] as u16;
        }

        fence(Ordering::Release);

        unsafe {
            (*self.avail).idx += 1;
        }

        fence(Ordering::Release);

        *virtio_reg(self.id, VIRTIO_MMIO_QUEUE_NOTIFY) = 0;

        while buf.disk == 1 {}

        self.info[idx[0]].b = 0 as *mut Buf;
        self.free_chain(idx[0]);
    }

    pub fn read(&mut self, sector: usize) -> [u8; 512] {
        assert!(sector < self.size);

        let mut buf = Buf {
            valid: 0,
            disk: 0,
            dev: 0,
            sector: sector as u32,
            lock: Lock::new(),
            refcnt: 0,
            prev: 0 as *mut Buf,
            next: 0 as *mut Buf,
            data: [0; 512],
        };

        self.virtio_disk_rw(&mut buf, false);

        buf.data

    }

    pub fn write(&mut self, sector: usize, data: &[u8; 512]) {
        assert!(sector < self.size);

        let mut buf = Buf {
            valid: 0,
            disk: 0,
            dev: 0,
            sector: sector as u32,
            lock: Lock::new(),
            refcnt: 0,
            prev: 0 as *mut Buf,
            next: 0 as *mut Buf,
            data: data.clone(),
        };

        self.virtio_disk_rw(&mut buf, true);

    }

    pub const fn size(&self) -> usize {
        self.size
    }
}

pub fn scan_for_disks() -> Vec<&'static mut Disk> {
    let mut vec = Vec::new();
    for id in 0..=MAX_VIRTIO_ID {
        if let Some(disk) = get_disk_at(id) {
            vec.push(disk);
        }
    }

    vec
}

static mut DISKS: [*mut Disk; (MAX_VIRTIO_ID + 1) as usize] = [0 as *mut Disk; (MAX_VIRTIO_ID + 1) as usize];

pub fn disk_irq(irq: u32) {
    let id = irq as u64;

    let disk_ptr = unsafe { DISKS[id as usize] };
    if disk_ptr == 0 as *mut Disk {
        return;
    }

    let disk = unsafe { &mut *disk_ptr };

    *virtio_reg(id, VIRTIO_MMIO_INTERRUPT_ACK) = *virtio_reg(id, VIRTIO_MMIO_INTERRUPT_STATUS) & 0x3;

    fence(Ordering::Release);

    let used = unsafe { &mut *disk.used };

    while disk.used_idx != used.idx {
        fence(Ordering::Release);
        let id = used.ring[disk.used_idx as usize % NUM].id;

        assert_eq!(disk.info[id as usize].status, 0);

        let buf = unsafe { &mut *disk.info[id as usize].b };
        buf.disk = 0;

        disk.used_idx += 1;
    }
}
