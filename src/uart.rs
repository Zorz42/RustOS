use std::println;
use crate::virtio::{virtio_reg};

pub fn uart_init() {
    let id = 1;
    *virtio_reg(id, 1) = 0;
    *virtio_reg(id, 3) = 1 << 7;
    *virtio_reg(id, 0) = 3;
    *virtio_reg(id, 1) = 0;
    *virtio_reg(id, 3) = 3;
    *virtio_reg(id, 2) = 7;
    *virtio_reg(id, 1) = 3;
}