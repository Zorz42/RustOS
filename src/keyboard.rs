use std::{malloc, println};
use crate::memory::PAGE_SIZE;
use crate::timer::get_ticks;
use crate::virtio::definitions::{virtio_reg_read, virtio_reg_write, MmioOffset, MAX_VIRTIO_ID, VIRTIO_CONFIG_S_ACKNOWLEDGE, VIRTIO_CONFIG_S_DRIVER, VIRTIO_CONFIG_S_DRIVER_OK, VIRTIO_CONFIG_S_FEATURES_OK, VIRTIO_MAGIC, VIRTIO_RING_F_EVENT_IDX, VIRTIO_RING_SIZE, VRING_DESC_F_WRITE};

const EVENT_BUFFER_ELEMENTS: usize = 64;

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
    event_queue:  *mut Queue,
    status_queue: *mut Queue,
    event_idx:          u16,
    event_ack_used_idx: u16,
    event_buffer: *mut Event,
    status_ack_used_idx: u16,
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

        let status = 0;
        virtio_reg_write(id, MmioOffset::Status, status);

        let status = status | VIRTIO_CONFIG_S_ACKNOWLEDGE;
        virtio_reg_write(id, MmioOffset::Status, status);

        let status = status | VIRTIO_CONFIG_S_DRIVER;
        virtio_reg_write(id, MmioOffset::Status, status);

        let host_features = virtio_reg_read(id, MmioOffset::DriverFeatures);
        let host_features = host_features & !(1 << VIRTIO_RING_F_EVENT_IDX);
        virtio_reg_write(id, MmioOffset::DriverFeaturesSel, host_features);

        let status = status | VIRTIO_CONFIG_S_FEATURES_OK;
        virtio_reg_write(id, MmioOffset::Status, status);

        let read_status = virtio_reg_read(id, MmioOffset::Status);
        assert_eq!(read_status, status);

        let qnmax = virtio_reg_read(id, MmioOffset::QueueNumMax);
        assert!(VIRTIO_RING_SIZE as u32 <= qnmax);

        let mut keyboard = Keyboard {
            event_queue: 0 as *mut Queue,
            status_queue: 0 as *mut Queue,
            event_idx: 0,
            event_ack_used_idx: 0,
            event_buffer: malloc(EVENT_BUFFER_ELEMENTS * size_of::<Event>()) as *mut Event,
            status_ack_used_idx: 0,
        };

        virtio_reg_write(id, MmioOffset::QueueSel, 0);
        keyboard.event_queue = malloc(size_of::<Queue>()) as *mut Queue;
        virtio_reg_write(id, MmioOffset::QueueDescLow, keyboard.event_queue as u32);
        virtio_reg_write(id, MmioOffset::QueueDescHigh, (keyboard.event_queue as u64 >> 32) as u32);
        virtio_reg_write(id, MmioOffset::QueueNum, VIRTIO_RING_SIZE as u32);

        virtio_reg_write(id, MmioOffset::QueueSel, 1);
        keyboard.status_queue = malloc(size_of::<Queue>()) as *mut Queue;
        virtio_reg_write(id, MmioOffset::QueueDescLow, keyboard.status_queue as u32);
        virtio_reg_write(id, MmioOffset::QueueDescHigh, (keyboard.status_queue as u64 >> 32) as u32);
        virtio_reg_write(id, MmioOffset::QueueNum, VIRTIO_RING_SIZE as u32);

        let status = status | VIRTIO_CONFIG_S_DRIVER_OK;
        virtio_reg_write(id, MmioOffset::Status, status);

        for i in 0..EVENT_BUFFER_ELEMENTS {
            keyboard.repopulate_event(i);
        }

        loop {
            if get_ticks() % 1000 == 0 {
                println!("Checking... {}", self.);
            }
        }

        Some(keyboard)
    }

    fn repopulate_event(&mut self, buffer: usize) {
        unsafe {
            let desc = Descriptor {
                addr: self.event_buffer.add(buffer) as u64,
                len: size_of::<Event>() as u32,
                flags: VRING_DESC_F_WRITE,
                next: 0
            };
            let head = self.event_idx;
            (*self.event_queue).desc[self.event_idx as usize] = desc;
            self.event_idx = (self.event_idx + 1) % VIRTIO_RING_SIZE as u16;
            (*self.event_queue).avail.ring[(*self.event_queue).avail.idx as usize % VIRTIO_RING_SIZE] = head;
            (*self.event_queue).avail.idx = (*self.event_queue).avail.idx.wrapping_add(1);
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
