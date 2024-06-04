use crate::spinlock::Lock;
use crate::virtio::{
    virtio_reg, VirtioBlqReq, VirtqAvail, VirtqDesc, VirtqUsed, MAX_VIRTIO_ID, NUM, VIRTIO_BLK_F_CONFIG_WCE, VIRTIO_BLK_F_MQ, VIRTIO_BLK_F_RO, VIRTIO_BLK_F_SCSI, VIRTIO_CONFIG_S_ACKNOWLEDGE,
    VIRTIO_CONFIG_S_DRIVER, VIRTIO_F_ANY_LAYOUT, VIRTIO_MMIO_DEVICE_FEATURES, VIRTIO_MMIO_DEVICE_ID, VIRTIO_MMIO_DRIVER_FEATURES, VIRTIO_MMIO_MAGIC_VALUE, VIRTIO_MMIO_STATUS, VIRTIO_MMIO_VENDOR_ID,
    VIRTIO_MMIO_VERSION, VIRTIO_RING_F_EVENT_IDX, VIRTIO_RING_F_INDIRECT_DESC,
};
use std::{Box, Vec};

const BSIZE: usize = 1024;

struct Buf {
    valid: i32, // has data been read from disk?
    disk: i32,  // does disk "own" buf?
    dev: u32,
    blockno: u32,
    lock: Lock,
    refcnt: u32,
    prev: *mut Buf, // LRU cache list
    next: *mut Buf,
    data: [u8; BSIZE],
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
}

pub fn get_disk_at(id: u64) -> Option<Box<Disk>> {
    if (*virtio_reg(id, VIRTIO_MMIO_MAGIC_VALUE) != 0x74726976
        || *virtio_reg(id, VIRTIO_MMIO_VERSION) != 2
        || *virtio_reg(id, VIRTIO_MMIO_DEVICE_ID) != 2
        || *virtio_reg(id, VIRTIO_MMIO_VENDOR_ID) != 0x554d4551)
    {
        return None;
    }

    let mut disk = Box::new(Disk {
        desc: 0 as *mut VirtqDesc,
        avail: 0 as *mut VirtqAvail,
        used: 0 as *mut VirtqUsed,
        free: [false; NUM],
        used_idx: 0,
        info: [Info { b: 0 as *mut Buf, status: 0 }; NUM],
        ops: [VirtioBlqReq { typ: 0, reserved: 0, sector: 0 }; NUM],
        vdisk_lock: Lock::new(),
        id,
    });

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

    Some(disk)
}

pub fn scan_for_disks() -> Vec<Box<Disk>> {
    let mut vec = Vec::new();
    for id in 0..=MAX_VIRTIO_ID {
        if let Some(disk) = get_disk_at(id) {
            vec.push(disk);
        }
    }

    vec
}
