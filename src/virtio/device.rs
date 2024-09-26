use core::ptr::write_bytes;
use crate::disk::disk::Disk;
use crate::memory::{alloc_page, PAGE_SIZE};
use crate::spinlock::Lock;
use crate::virtio::definitions::{virtio_reg_read, virtio_reg_write, MmioOffset, VirtqAvail, VirtqDesc, VirtqUsed, MAX_VIRTIO_ID, NUM, VIRTIO_CONFIG_S_ACKNOWLEDGE, VIRTIO_CONFIG_S_DRIVER, VIRTIO_CONFIG_S_DRIVER_OK, VIRTIO_CONFIG_S_FEATURES_OK, VIRTIO_F_ANY_LAYOUT, VIRTIO_MAGIC, VIRTIO_RING_F_EVENT_IDX, VIRTIO_RING_F_INDIRECT_DESC, VRING_DESC_F_NEXT};

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

    pub fn get_config_address(&self) -> *mut u8 {
        virtio_reg_read(self.virtio_id, MmioOffset::Config) as *mut u8
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
}