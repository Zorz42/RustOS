use crate::virtio::definitions::{MAX_VIRTIO_ID, VIRTIO_RING_F_EVENT_IDX};
use crate::virtio::device::VirtioDevice;

struct Keyboard {
    virtio_device: &'static mut VirtioDevice,
}

fn get_keyboard_at(id: u64) -> Option<Keyboard> {
    let mut features = !0;
    features &= !(1 << VIRTIO_RING_F_EVENT_IDX);
    let device = VirtioDevice::get_device_at(id, 2, 18, 0x554d4551, features);

    device.map(|device| Keyboard {
        virtio_device: device,
    })
}

impl Keyboard {

}

static mut KEYBOARD: Option<Keyboard> = None;

pub fn init_keyboard() {
    for id in 0..MAX_VIRTIO_ID {
        if let Some(keyboard) = get_keyboard_at(id) {
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
