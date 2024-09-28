use core::arch::asm;
use core::hint::black_box;
use core::mem::MaybeUninit;
use core::ptr::{addr_of, read_volatile, write_bytes, write_volatile};
use core::sync::atomic::{fence, Ordering};
use std::{print, println, Box};
use crate::gpu::refresh_screen;
use crate::memory::{alloc_page, virt_to_phys, VirtAddr, PAGE_SIZE};
use crate::riscv::get_core_id;
use crate::spinlock::Lock;
use crate::timer::get_ticks;
use crate::virtio::definitions::{virtio_reg_addr, virtio_reg_read, virtio_reg_write, MmioOffset, VirtqAvail, VirtqDesc, VirtqUsed, MAX_VIRTIO_ID, NUM, VIRTIO_CONFIG_S_ACKNOWLEDGE, VIRTIO_CONFIG_S_DRIVER, VIRTIO_CONFIG_S_DRIVER_OK, VIRTIO_CONFIG_S_FEATURES_OK, VIRTIO_F_ANY_LAYOUT, VIRTIO_MAGIC, VIRTIO_RING_F_EVENT_IDX, VIRTIO_RING_F_INDIRECT_DESC, VRING_DESC_F_NEXT, VRING_DESC_F_WRITE};

pub struct VirtioDevice {
    // three virtqueues
    desc: *mut VirtqDesc,
    avail: *mut VirtqAvail,
    used: *mut VirtqUsed,

    // our own book-keeping.
    free: [bool; NUM],
    used_idx: u16,
    info: [bool; NUM],
    lock: Lock,
    virtio_id: u64,
    irq_waiting: bool,
}

const ARRAY_REPEAT_VALUE: Option<VirtioDevice> = None;
static mut DEVICES: [Option<VirtioDevice>; MAX_VIRTIO_ID as usize] = [ARRAY_REPEAT_VALUE; MAX_VIRTIO_ID as usize];

impl VirtioDevice {
    pub fn get_device_at(id: u64, expected_version: u32, expected_device_id: u32, expected_vendor_id: u32, features: u32) -> Option<&'static mut Self> {
        if virtio_reg_read(id, MmioOffset::MagicValue) != VIRTIO_MAGIC
            || virtio_reg_read(id, MmioOffset::Version) != expected_version
            || virtio_reg_read(id, MmioOffset::DeviceId) != expected_device_id
            || virtio_reg_read(id, MmioOffset::VendorId) != expected_vendor_id
        {
            return None;
        }


        unsafe {
            DEVICES[id as usize] = Some(Self {
                desc: 0 as *mut VirtqDesc,
                avail: 0 as *mut VirtqAvail,
                used: 0 as *mut VirtqUsed,
                free: [true; NUM],
                used_idx: 0,
                info: [false; NUM],
                lock: Lock::new(),
                virtio_id: id,
                irq_waiting: false,
            });
        }

        let device = unsafe { DEVICES[id as usize].as_mut().unwrap() };

        let mut status = 0;

        status |= VIRTIO_CONFIG_S_ACKNOWLEDGE;

        virtio_reg_write(id, MmioOffset::Status, status);

        status |= VIRTIO_CONFIG_S_DRIVER;
        virtio_reg_write(id, MmioOffset::Status, status);

        // negotiate features
        let mut featuresr = virtio_reg_read(id, MmioOffset::DeviceFeatures);
        featuresr &= features;
        featuresr &= !(1 << VIRTIO_F_ANY_LAYOUT);
        featuresr &= !(1 << VIRTIO_RING_F_EVENT_IDX);
        featuresr &= !(1 << VIRTIO_RING_F_INDIRECT_DESC);
        virtio_reg_write(id, MmioOffset::DriverFeatures, featuresr);

        status |= VIRTIO_CONFIG_S_FEATURES_OK;
        virtio_reg_write(id, MmioOffset::Status, status);

        // reread and check
        status = virtio_reg_read(id, MmioOffset::Status);
        assert_eq!(status & VIRTIO_CONFIG_S_FEATURES_OK, VIRTIO_CONFIG_S_FEATURES_OK);

        virtio_reg_write(id, MmioOffset::QueueSel, 0);

        assert_eq!(virtio_reg_read(id, MmioOffset::QueueReady), 0);

        assert!(virtio_reg_read(id, MmioOffset::QueueNumMax) >= NUM as u32);

        device.desc = alloc_page() as *mut VirtqDesc;
        device.avail = alloc_page() as *mut VirtqAvail;
        device.used = alloc_page() as *mut VirtqUsed;

        unsafe {
            write_bytes(device.desc as *mut u8, 0, PAGE_SIZE as usize);
            write_bytes(device.avail as *mut u8, 0, PAGE_SIZE as usize);
            write_bytes(device.used as *mut u8, 0, PAGE_SIZE as usize);
        }

        virtio_reg_write(id, MmioOffset::QueueNum, NUM as u32);

        virtio_reg_write(id, MmioOffset::QueueDescLow, (device.desc as u64 & 0xFFFFFFFF) as u32);
        virtio_reg_write(id, MmioOffset::QueueDescHigh, ((device.desc as u64 >> 32) & 0xFFFFFFFF) as u32);
        virtio_reg_write(id, MmioOffset::DriverDescLow, (device.avail as u64 & 0xFFFFFFFF) as u32);
        virtio_reg_write(id, MmioOffset::DriverDescHigh, ((device.avail as u64 >> 32) & 0xFFFFFFFF) as u32);
        virtio_reg_write(id, MmioOffset::DeviceDescLow, (device.used as u64 & 0xFFFFFFFF) as u32);
        virtio_reg_write(id, MmioOffset::DeviceDescHigh, ((device.used as u64 >> 32) & 0xFFFFFFFF) as u32);

        // queue is ready
        virtio_reg_write(id, MmioOffset::QueueReady, 1);

        // we are completely ready
        status |= VIRTIO_CONFIG_S_DRIVER_OK;
        virtio_reg_write(id, MmioOffset::Status, status);

        Some(device)
    }

    pub const fn get_config_address(&self) -> *mut u8 {
        virtio_reg_addr(self.virtio_id, MmioOffset::Config)
    }

    fn alloc_1desc(&mut self) -> Option<usize> {
        for i in 0..NUM {
            if self.free[i] {
                self.free[i] = false;
                return Some(i);
            }
        }
        None
    }

    fn alloc_2desc(&mut self) -> Option<[usize; 2]> {
        let mut res = [0; 2];

        for (i, r) in res.iter_mut().enumerate() {
            let desc = self.alloc_1desc();
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

    fn alloc_3desc(&mut self) -> Option<[usize; 3]> {
        let mut res = [0; 3];

        for (i, r) in res.iter_mut().enumerate() {
            let desc = self.alloc_1desc();
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

    fn virtio_send(&mut self, idx: usize) {
        self.lock.spinlock();

        unsafe {
            let idx2 = (*self.avail).idx as usize;
            (*self.avail).ring[idx2 % NUM] = idx as u16;
        }

        fence(Ordering::Release);

        unsafe {
            (*self.avail).idx += 1;
        }

        fence(Ordering::Release);

        unsafe { write_volatile(&mut self.info[idx], true); }

        virtio_reg_write(self.virtio_id, MmioOffset::QueueNotify, 0);

        self.lock.unlock();

        let start_time = get_ticks();

        if self.irq_waiting {
            virtio_irq(self.virtio_id as u32 + 1);
            self.irq_waiting = false;
        }

        while unsafe { read_volatile(&self.info[idx]) } {
            assert!(get_ticks() - start_time < 3000, "virtio_send timeout");

            self.irq_waiting = black_box(self.irq_waiting);
            unsafe {
                asm!("wfi");
            }
        }
    }

    pub fn virtio_send_rww<Arg1,Res1,Res2>(&mut self, arg1: &Arg1) -> (Res1, Res2) {
        self.lock.spinlock();

        let idx = self.alloc_3desc().unwrap();

        let desc0 = self.get_desc(idx[0]);
        desc0.addr = virt_to_phys(arg1 as *const Arg1 as VirtAddr).unwrap();
        desc0.len = size_of::<Arg1>() as u32;
        desc0.flags = VRING_DESC_F_NEXT;
        desc0.next = idx[1] as u16;

        let res1 = unsafe { MaybeUninit::zeroed().assume_init() };

        let desc1 = self.get_desc(idx[1]);
        desc1.addr = addr_of!(res1) as u64;
        desc1.len = size_of::<Res1>() as u32;
        desc1.flags = VRING_DESC_F_WRITE | VRING_DESC_F_NEXT;
        desc1.next = idx[2] as u16;

        let res2 = unsafe { MaybeUninit::zeroed().assume_init() };

        let desc2 = self.get_desc(idx[2]);
        desc2.addr = addr_of!(res2) as u64;
        desc2.len = size_of::<Res2>() as u32;
        desc2.flags = VRING_DESC_F_WRITE;
        desc2.next = 0;

        self.lock.unlock();

        self.virtio_send(idx[0]);

        self.lock.spinlock();
        self.free_chain(idx[0]);
        self.lock.unlock();

        black_box(arg1); // we need that data all the way through
        black_box((res1, res2)) // because virtio does stuff to it
    }

    pub fn virtio_send_rrw<Arg1,Arg2,Res1>(&mut self, arg1: &Arg1, arg2: &Arg2) -> Res1 {
        self.lock.spinlock();

        let idx = self.alloc_3desc().unwrap();

        let desc0 = self.get_desc(idx[0]);
        desc0.addr = virt_to_phys(arg1 as *const Arg1 as VirtAddr).unwrap();
        desc0.len = size_of::<Arg1>() as u32;
        desc0.flags = VRING_DESC_F_NEXT;
        desc0.next = idx[1] as u16;

        let desc1 = self.get_desc(idx[1]);
        desc1.addr = virt_to_phys(arg2 as *const Arg2 as VirtAddr).unwrap();
        desc1.len = size_of::<Arg2>() as u32;
        desc1.flags = VRING_DESC_F_NEXT;
        desc1.next = idx[2] as u16;

        let res1 = unsafe { MaybeUninit::zeroed().assume_init() };

        let desc2 = self.get_desc(idx[2]);
        desc2.addr = addr_of!(res1) as u64;
        desc2.len = size_of::<Res1>() as u32;
        desc2.flags = VRING_DESC_F_WRITE;
        desc2.next = 0;

        self.lock.unlock();

        self.virtio_send(idx[0]);

        self.lock.spinlock();
        self.free_chain(idx[0]);
        self.lock.unlock();

        black_box(arg1); // we need that data all the way through
        black_box(arg2);
        black_box(res1) // because virtio does stuff to it
    }

    pub fn virtio_send_rw<Arg1,Res1>(&mut self, arg1: &Arg1) -> Res1 {
        self.lock.spinlock();

        let idx = self.alloc_2desc().unwrap();

        let desc0 = self.get_desc(idx[0]);
        desc0.addr = virt_to_phys(arg1 as *const Arg1 as VirtAddr).unwrap();
        desc0.len = size_of::<Arg1>() as u32;
        desc0.flags = VRING_DESC_F_NEXT;
        desc0.next = idx[1] as u16;

        let res1 = unsafe { MaybeUninit::zeroed().assume_init() };

        let desc1 = self.get_desc(idx[1]);
        desc1.addr = addr_of!(res1) as u64;
        desc1.len = size_of::<Res1>() as u32;
        desc1.flags = VRING_DESC_F_WRITE;
        desc1.next = 0;

        self.lock.unlock();

        self.virtio_send(idx[0]);

        self.lock.spinlock();
        self.free_chain(idx[0]);
        self.lock.unlock();

        black_box(arg1); // we need that data all the way through
        black_box(res1) // because virtio does stuff to it
    }
}

pub fn virtio_irq(irq: u32) {
    if irq == 0 || irq > MAX_VIRTIO_ID as u32 {
        return;
    }
    let id = irq as u64 - 1;

    if let Some(device) = unsafe { DEVICES[id as usize].as_mut() } {
        if device.lock.locked_by() == get_core_id() as i32 {
            device.irq_waiting = true;
            return;
        }

        device.lock.spinlock();

        virtio_reg_write(id, MmioOffset::InterruptAck, virtio_reg_read(id, MmioOffset::InterruptStatus) & 0x3);

        fence(Ordering::Release);

        let used = unsafe { &mut *device.used };

        while device.used_idx != used.idx {
            fence(Ordering::Release);
            let id = used.ring[device.used_idx as usize % NUM].id;

            unsafe {
                write_volatile(&mut device.info[id as usize], false);
            }

            device.used_idx += 1;
        }

        device.lock.unlock();
    }
}