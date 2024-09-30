use core::intrinsics::write_bytes;
use std::{malloc, println};
use crate::memory::{alloc_page, virt_to_phys, PhysAddr, VirtAddr, PAGE_SIZE};
use crate::spinlock::Lock;
use crate::timer::get_ticks;
use crate::virtio::definitions::{virtio_reg_read, virtio_reg_write, MmioOffset, VirtqAvail, VirtqDesc, VirtqUsed, MAX_VIRTIO_ID, NUM, VIRTIO_CONFIG_S_ACKNOWLEDGE, VIRTIO_CONFIG_S_DRIVER, VIRTIO_CONFIG_S_DRIVER_OK, VIRTIO_CONFIG_S_FEATURES_OK, VIRTIO_F_ANY_LAYOUT, VIRTIO_MAGIC, VIRTIO_RING_F_EVENT_IDX, VIRTIO_RING_F_INDIRECT_DESC, VRING_DESC_F_WRITE};

const EVENT_BUFFER_ELEMENTS: usize = 8;
pub const VIRTIO_RING_SIZE: usize = 8;

#[repr(C)]
pub struct Descriptor {
    pub addr:  u64,
    pub len:   u32,
    pub flags: u16,
    pub next:  u16,
}

#[repr(C)]
pub struct Available {
    pub flags: u16,
    pub idx:   u16,
    pub ring:  [u16; VIRTIO_RING_SIZE],
    pub event: u16,
}

#[repr(C)]
pub struct UsedElem {
    pub id:  u32,
    pub len: u32,
}

#[repr(C)]
pub struct Used {
    pub flags: u16,
    pub idx:   u16,
    pub ring:  [UsedElem; VIRTIO_RING_SIZE],
    pub event: u16,
}

#[repr(C)]
pub struct Queue {
    pub desc:  [Descriptor; VIRTIO_RING_SIZE],
    pub avail: Available,
    // Calculating padding, we need the used ring to start on a page boundary. We take the page size, subtract the
    // amount the descriptor ring takes then subtract the available structure and ring.
    pub padding0: [u8; PAGE_SIZE as usize - size_of::<Descriptor>() * VIRTIO_RING_SIZE - size_of::<Available>()],
    pub used:     Used,
}

#[repr(u16)]
#[derive(Copy, Clone)]
pub enum EventType {
    Syn = 0x00,
    Key = 0x01,
    Rel = 0x02,
    Abs = 0x03,
    Msc = 0x04,
    Sw = 0x05,
    Led = 0x11,
    Snd = 0x12,
    Rep = 0x14,
    Ff = 0x15,
    Pwr = 0x16,
    FfStatus = 0x17,
    Max = 0x1f,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub code: u16,
    pub value: u32,
}

struct Keyboard {
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

    event_buffer: *mut Event,
    event_idx: u16,
}

impl Keyboard {
    fn get_keyboard_at(id: u64) -> Option<Keyboard> {
        if virtio_reg_read(id, MmioOffset::MagicValue) != VIRTIO_MAGIC
            || virtio_reg_read(id, MmioOffset::Version) != 2
            || virtio_reg_read(id, MmioOffset::DeviceId) != 18
            || virtio_reg_read(id, MmioOffset::VendorId) != 0x554d4551
        {
            return None;
        }

        let mut device = Self {
            desc: 0 as *mut VirtqDesc,
            avail: 0 as *mut VirtqAvail,
            used: 0 as *mut VirtqUsed,
            free: [true; NUM],
            used_idx: 0,
            info: [false; NUM],
            lock: Lock::new(),
            virtio_id: id,
            irq_waiting: false,
            event_buffer: malloc(EVENT_BUFFER_ELEMENTS * size_of::<Event>()) as *mut Event,
            event_idx: 0,
        };

        let mut status = 0;

        status |= VIRTIO_CONFIG_S_ACKNOWLEDGE;

        virtio_reg_write(id, MmioOffset::Status, status);

        status |= VIRTIO_CONFIG_S_DRIVER;
        virtio_reg_write(id, MmioOffset::Status, status);

        // negotiate features
        let mut featuresr = virtio_reg_read(id, MmioOffset::DeviceFeatures);
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
        assert_eq!(virtio_reg_read(id, MmioOffset::Status), status);

        for i in 0..EVENT_BUFFER_ELEMENTS {
            device.repopulate_event(i);
        }

        loop {
            if get_ticks() % 1000 == 0 {
                println!("Checking...");
                for i in 0..EVENT_BUFFER_ELEMENTS {
                    unsafe {
                        let obj = device.event_buffer.add(i).read_volatile();
                        if obj.code != 0 || obj.value != 0 {
                            println!("Event: {} {}", obj.code, obj.value);
                        }
                    }
                }
            }
        }

        Some(device)
    }

    fn repopulate_event(&mut self, buffer: usize) {
        unsafe {
            let desc = VirtqDesc {
                addr: virt_to_phys(self.event_buffer.add(buffer) as u64 as VirtAddr).unwrap(),
                len: size_of::<Event>() as u32,
                flags: VRING_DESC_F_WRITE,
                next: 0
            };
            let head = self.event_idx;
            self.desc.add(self.event_idx as usize).write_volatile(desc);
            self.event_idx = (self.event_idx + 1) % VIRTIO_RING_SIZE as u16;
            (*self.avail).ring[(*self.avail).idx as usize % VIRTIO_RING_SIZE] = head;
            (*self.avail).idx = (*self.avail).idx.wrapping_add(1);
        }
    }
}

static mut KEYBOARD: Option<Keyboard> = None;

pub fn init_keyboard() {
    for id in 0..MAX_VIRTIO_ID {
        if let Some(keyboard) = Keyboard::get_keyboard_at(id) {
            unsafe {
                assert!(KEYBOARD.is_none());
                KEYBOARD = Some(keyboard);
            }
            break;
        }
    }
    unsafe {
        assert!(KEYBOARD.is_some());
    }
}
