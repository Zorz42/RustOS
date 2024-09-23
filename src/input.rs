use crate::virtio::{MAX_VIRTIO_ID, MmioOffset, VIRTIO_MAGIC, virtio_reg_read};

struct Device {

}

fn get_input_device_at(id: u64) -> Option<Device> {
    if virtio_reg_read(id, MmioOffset::MagicValue) != VIRTIO_MAGIC

        {
        return None;
    }

    None
}

const ARRAY_REPEAT_VALUE: Option<Device> = None;
static mut DEVICES: [Option<Device>; MAX_VIRTIO_ID as usize] = [ARRAY_REPEAT_VALUE; MAX_VIRTIO_ID as usize];

pub fn init_input() {
    for id in 0..8 {
        unsafe {
            DEVICES[id as usize] = get_input_device_at(id);
        }
    }
}