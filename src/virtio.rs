// virtio mmio control registers, mapped starting at 0x10001000.
pub const VIRTIO_MMIO_BASE: u64 =             0x10001000;
pub const VIRTIO_MMIO_MAGIC_VALUE: u64 =      0x000; // 0x74726976
pub const VIRTIO_MMIO_VERSION: u64 =          0x004; // version; should be 2
pub const VIRTIO_MMIO_DEVICE_ID: u64 =        0x008; // device type; 1 is net, 2 is disk
pub const VIRTIO_MMIO_VENDOR_ID: u64 =        0x00c; // 0x554d4551
pub const VIRTIO_MMIO_DEVICE_FEATURES: u64 =  0x010;
pub const VIRTIO_MMIO_DRIVER_FEATURES: u64 =  0x020;
pub const VIRTIO_MMIO_QUEUE_SEL: u64 =        0x030; // select queue, write-only
pub const VIRTIO_MMIO_QUEUE_NUM_MAX: u64 =    0x034; // max size of current queue, read-only
pub const VIRTIO_MMIO_QUEUE_NUM: u64 =        0x038; // size of current queue, write-only
pub const VIRTIO_MMIO_QUEUE_READY: u64 =      0x044; // ready bit
pub const VIRTIO_MMIO_QUEUE_NOTIFY: u64 =     0x050; // write-only
pub const VIRTIO_MMIO_INTERRUPT_STATUS: u64 = 0x060; // read-only
pub const VIRTIO_MMIO_INTERRUPT_ACK: u64 =    0x064; // write-only
pub const VIRTIO_MMIO_STATUS: u64 =           0x070; // read/write
pub const VIRTIO_MMIO_QUEUE_DESC_LOW: u64 =   0x080; // physical address for descriptor table, write-only
pub const VIRTIO_MMIO_QUEUE_DESC_HIGH: u64 =  0x084;
pub const VIRTIO_MMIO_DRIVER_DESC_LOW: u64 =  0x090; // physical address for available ring, write-only
pub const VIRTIO_MMIO_DRIVER_DESC_HIGH: u64 = 0x094;
pub const VIRTIO_MMIO_DEVICE_DESC_LOW: u64 =  0x0a0; // physical address for used ring, write-only
pub const VIRTIO_MMIO_DEVICE_DESC_HIGH: u64 = 0x0a4;

pub fn virtio_reg(reg: u64) -> &'static mut u32 {
    let addr = VIRTIO_MMIO_BASE + reg;
    unsafe { &mut *(addr as *mut u32) }
}

// status register bits
pub const VIRTIO_CONFIG_S_ACKNOWLEDGE: u64 = 1;
pub const VIRTIO_CONFIG_S_DRIVER: u64 =      1 << 1;
pub const VIRTIO_CONFIG_S_DRIVER_OK: u64 =   1 << 2;
pub const VIRTIO_CONFIG_S_FEATURES_OK: u64 = 1 << 3;

// device feature bits
pub const VIRTIO_BLK_F_RO: u64 =             5;	 // Disk is read-only
pub const VIRTIO_BLK_F_SCSI: u64 =           7;	 // Supports scsi command passthru
pub const VIRTIO_BLK_F_CONFIG_WCE: u64 =     11; // Writeback mode available in config
pub const VIRTIO_BLK_F_MQ: u64 =             12; // support more than one vq
pub const VIRTIO_F_ANY_LAYOU: u64 =          27;
pub const VIRTIO_RING_F_INDIRECT_DESC: u64 = 28;
pub const VIRTIO_RING_F_EVENT_IDX: u64 =     29;

// this many virtio descriptors.
// must be a power of two.
pub const NUM: usize = 8;

// a single descriptor, from the spec.
#[repr(C)]
pub struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

pub const VRING_DESC_F_NEXT: u64 = 1; // chained with another descriptor
pub const VRING_DESC_F_WRITE: u64 = 2; // device writes (vs read)

// the (entire) avail ring, from the spec.
#[repr(C)]
pub struct VirtqAvail {
    flags: u16, // always zero
    idx: u16,   // driver will write ring[idx] next
    ring: [u16; NUM], // descriptor numbers of chain heads
    unused: u16,
}

// one entry in the "used" ring, with which the
// device tells the driver about completed requests.
pub struct VirtqUsedElem {
    id: u32,   // index of start of completed descriptor chain
    len: u32,
}

pub struct VirtqUsed {
    flags: u16, // always zero
    idx: u16,   // device increments when it adds a ring[] entry
    ring: [VirtqUsedElem; NUM],
}

// these are specific to virtio block devices, e.g. disks,
// described in Section 5.2 of the spec.

pub const VIRTIO_BLK_T_IN: u64 = 0; // read the disk
pub const VIRTIO_BLK_T_OUT: u64 = 1; // write the disk

// the format of the first descriptor in a disk request.
// to be followed by two more descriptors containing
// the block, and a one-byte status.
pub struct VirtioBlqReq {
    typ: u32, // VIRTIO_BLK_T_IN or ..._OUT
    reserved: u32,
    sector: u64,
}