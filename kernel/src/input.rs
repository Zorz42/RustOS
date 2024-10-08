use core::cmp::PartialEq;
use core::intrinsics::write_bytes;
use core::sync::atomic::{fence, Ordering};
use std::{println, Vec};
use crate::memory::{alloc_page, virt_to_phys, VirtAddr, PAGE_SIZE};
use crate::riscv::get_core_id;
use crate::spinlock::Lock;
use crate::virtio::definitions::{virtio_reg_read, virtio_reg_write, MmioOffset, VirtqAvail, VirtqDesc, VirtqUsed, MAX_VIRTIO_ID, NUM, VIRTIO_CONFIG_S_ACKNOWLEDGE, VIRTIO_CONFIG_S_DRIVER, VIRTIO_CONFIG_S_DRIVER_OK, VIRTIO_CONFIG_S_FEATURES_OK, VIRTIO_F_ANY_LAYOUT, VIRTIO_MAGIC, VIRTIO_RING_F_EVENT_IDX, VIRTIO_RING_F_INDIRECT_DESC, VRING_DESC_F_WRITE};
use crate::virtio::device::VirtioDevice;

const EVENT_BUFFER_ELEMENTS: usize = 128;
const VIRTIO_RING_SIZE: usize = 128;

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
#[derive(Copy, Clone, Debug, PartialEq)]
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
#[derive(Copy, Clone, Debug)]
pub struct InputEvent {
    pub event_type: EventType,
    pub code: u16,
    pub value: i32,
}

const EVENT_QUEUE_SIZE: usize = 128;

struct VirtioInputDevice {
    // three virtqueues
    desc: *mut VirtqDesc,
    avail: *mut VirtqAvail,
    used: *mut VirtqUsed,

    // our own book-keeping.
    used_idx: u16,
    lock: Lock,
    virtio_id: u64,
    irq_waiting: bool,

    event_buffer: [*mut InputEvent; EVENT_BUFFER_ELEMENTS],
    event_idx: u16,

    events_queue: [InputEvent; EVENT_QUEUE_SIZE],
    events_queue_l: usize,
    events_queue_r: usize,
}

impl VirtioInputDevice {
    fn get_device_at(id: u64) -> Option<VirtioInputDevice> {
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
            used_idx: 0,
            lock: Lock::new(),
            virtio_id: id,
            irq_waiting: false,
            event_buffer: [0 as *mut InputEvent; EVENT_BUFFER_ELEMENTS],
            event_idx: 0,
            events_queue: [InputEvent {
                event_type: EventType::Syn,
                code: 0,
                value: 0,
            }; 128],
            events_queue_l: 0,
            events_queue_r: 0,
        };

        for i in 0..EVENT_BUFFER_ELEMENTS {
            device.event_buffer[i] = alloc_page() as *mut InputEvent;
        }

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

        device.lock.spinlock();
        for i in 0..EVENT_BUFFER_ELEMENTS {
            device.repopulate_event(i);
        }
        device.lock.unlock();
        device.catch_up();

        Some(device)
    }

    fn repopulate_event(&mut self, buffer: usize) {
        unsafe {
            let desc = VirtqDesc {
                addr: virt_to_phys(self.event_buffer[buffer] as VirtAddr).unwrap(),
                len: size_of::<InputEvent>() as u32,
                flags: VRING_DESC_F_WRITE,
                next: 0,
            };
            let head = self.event_idx;
            self.desc.add(self.event_idx as usize).write_volatile(desc);
            self.event_idx = (self.event_idx + 1) % VIRTIO_RING_SIZE as u16;
            (*self.avail).ring[(*self.avail).idx as usize % VIRTIO_RING_SIZE] = head;
            (*self.avail).idx = (*self.avail).idx.wrapping_add(1);
        }
    }

    fn catch_up(&mut self) {
        if self.irq_waiting {
            virtio_input_irq(self.virtio_id as u32);
            self.irq_waiting = false;
        }
    }

    fn get_from_queue(&mut self) -> Option<InputEvent> {
        if self.events_queue_l != self.events_queue_r {
            let res = self.events_queue[self.events_queue_l];
            self.events_queue_l = (self.events_queue_l + 1) % EVENT_QUEUE_SIZE;
            Some(res)
        } else {
            None
        }
    }

    fn receive_input(&mut self) -> Option<InputEvent> {
        self.lock.spinlock();
        while let Some(event) = self.get_from_queue() {
            if event.event_type != EventType::Syn {
                self.lock.unlock();
                self.catch_up();
                return Some(event);
            }
        };
        self.lock.unlock();
        self.catch_up();
        None
    }
}

const ARRAY_REPEAT_VALUE: Option<VirtioInputDevice> = None;
static mut DEVICES: [Option<VirtioInputDevice>; MAX_VIRTIO_ID as usize] = [ARRAY_REPEAT_VALUE; MAX_VIRTIO_ID as usize];

pub fn init_input_devices() {
    for id in 0..MAX_VIRTIO_ID {
        if let Some(input_device) = VirtioInputDevice::get_device_at(id) {
            unsafe {
                DEVICES[id as usize] = Some(input_device);
            }
        }
    }
}

pub fn check_for_virtio_input_event() -> Option<InputEvent> {
    for id in 0..MAX_VIRTIO_ID {
        if let Some(device) = unsafe { &mut DEVICES[id as usize] } {
            if let Some(event) = device.receive_input() {
                return Some(event);
            }
        }
    }
    None
}

pub fn virtio_input_irq(irq: u32) {
    if irq == 0 || irq > MAX_VIRTIO_ID as u32 {
        return;
    }
    let id = irq as u64 - 1;

    if let Some(device) = unsafe { &mut DEVICES[id as usize] } {
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

            let desc = unsafe { device.desc.add(id as usize).read_volatile() };
            let event_ptr = desc.addr as *const InputEvent;
            let event = unsafe { event_ptr.read_volatile() };

            device.events_queue[device.events_queue_r] = event;
            device.events_queue_r = (device.events_queue_r + 1) % EVENT_QUEUE_SIZE;

            device.repopulate_event(id as usize);

            device.used_idx += 1;
        }

        device.lock.unlock();
    } else {
        //println!("virtio_irq: invalid id {}", id);
    }
}

pub const fn keycode_to_char(keycode: u16) -> Option<char> {
    match keycode {
        0x02 => Some('1'),
        0x03 => Some('2'),
        0x04 => Some('3'),
        0x05 => Some('4'),
        0x06 => Some('5'),
        0x07 => Some('6'),
        0x08 => Some('7'),
        0x09 => Some('8'),
        0x0a => Some('9'),
        0x0b => Some('0'),
        0x10 => Some('q'),
        0x11 => Some('w'),
        0x12 => Some('e'),
        0x13 => Some('r'),
        0x14 => Some('t'),
        0x15 => Some('y'),
        0x16 => Some('u'),
        0x17 => Some('i'),
        0x18 => Some('o'),
        0x19 => Some('p'),
        0x1e => Some('a'),
        0x1f => Some('s'),
        0x20 => Some('d'),
        0x21 => Some('f'),
        0x22 => Some('g'),
        0x23 => Some('h'),
        0x24 => Some('j'),
        0x25 => Some('k'),
        0x26 => Some('l'),
        0x2c => Some('z'),
        0x2d => Some('x'),
        0x2e => Some('c'),
        0x2f => Some('v'),
        0x30 => Some('b'),
        0x31 => Some('n'),
        0x32 => Some('m'),
        0x39 => Some(' '),
        _ => None,
    }
}
