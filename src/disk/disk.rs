use crate::spinlock::Lock;
use crate::virtio::{NUM, VirtioBlqReq, VirtqAvail, VirtqDesc, VirtqUsed};

const BSIZE: usize = 1024;

struct Buf {
    valid: i32,   // has data been read from disk?
    disk: i32,    // does disk "own" buf?
    dev: u32,
    blockno: u32,
    lock: Lock,
    refcnt: u32,
    prev: *mut Buf, // LRU cache list
    next: *mut Buf,
    data: [u8; BSIZE],
}

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
    free: [bool; NUM],  // is a descriptor free?
    used_idx: u16, // we've looked this far in used[2..NUM].

    // track info about in-flight operations,
    // for use when completion interrupt arrives.
    // indexed by first descriptor index of chain.
    info: [Info; NUM],

    // disk command headers.
    // one-for-one with descriptors, for convenience.
    ops: [VirtioBlqReq; NUM],

    vdisk_lock: Lock,
}