pub const MAX_VIRTIO_ID: u64 = 8;

// virtio mmio control registers, mapped starting at 0x10000000 + 0x1000 * id.
pub const VIRTIO_MMIO_BASE: u64 = 0x10000000;
pub const VIRTIO_MMIO_MAGIC_VALUE: u64 = 0x000; // 0x74726976
pub const VIRTIO_MMIO_VERSION: u64 = 0x004; // version; should be 2
pub const VIRTIO_MMIO_DEVICE_ID: u64 = 0x008; // device type; 1 is net, 2 is disk
pub const VIRTIO_MMIO_VENDOR_ID: u64 = 0x00c; // 0x554d4551
pub const VIRTIO_MMIO_DEVICE_FEATURES: u64 = 0x010;
pub const VIRTIO_MMIO_DRIVER_FEATURES: u64 = 0x020;
pub const VIRTIO_MMIO_QUEUE_SEL: u64 = 0x030; // select queue, write-only
pub const VIRTIO_MMIO_QUEUE_NUM_MAX: u64 = 0x034; // max size of current queue, read-only
pub const VIRTIO_MMIO_QUEUE_NUM: u64 = 0x038; // size of current queue, write-only
pub const VIRTIO_MMIO_QUEUE_READY: u64 = 0x044; // ready bit
pub const VIRTIO_MMIO_QUEUE_NOTIFY: u64 = 0x050; // write-only
pub const VIRTIO_MMIO_INTERRUPT_STATUS: u64 = 0x060; // read-only
pub const VIRTIO_MMIO_INTERRUPT_ACK: u64 = 0x064; // write-only
pub const VIRTIO_MMIO_STATUS: u64 = 0x070; // read/write
pub const VIRTIO_MMIO_QUEUE_DESC_LOW: u64 = 0x080; // physical address for descriptor table, write-only
pub const VIRTIO_MMIO_QUEUE_DESC_HIGH: u64 = 0x084;
pub const VIRTIO_MMIO_DRIVER_DESC_LOW: u64 = 0x090; // physical address for available ring, write-only
pub const VIRTIO_MMIO_DRIVER_DESC_HIGH: u64 = 0x094;
pub const VIRTIO_MMIO_DEVICE_DESC_LOW: u64 = 0x0a0; // physical address for used ring, write-only
pub const VIRTIO_MMIO_DEVICE_DESC_HIGH: u64 = 0x0a4;

pub fn virtio_reg(id: u64, reg: u64) -> &'static mut u32 {
    let addr = VIRTIO_MMIO_BASE + 0x1000 * id + reg;
    unsafe { &mut *(addr as *mut u32) }
}

// status register bits
pub const VIRTIO_CONFIG_S_ACKNOWLEDGE: u32 = 1;
pub const VIRTIO_CONFIG_S_DRIVER: u32 = 1 << 1;
pub const VIRTIO_CONFIG_S_DRIVER_OK: u32 = 1 << 2;
pub const VIRTIO_CONFIG_S_FEATURES_OK: u32 = 1 << 3;

// device feature bits
pub const VIRTIO_BLK_F_RO: u32 = 5; // Disk is read-only
pub const VIRTIO_BLK_F_SCSI: u32 = 7; // Supports scsi command passthru
pub const VIRTIO_BLK_F_CONFIG_WCE: u32 = 11; // Writeback mode available in config
pub const VIRTIO_BLK_F_MQ: u32 = 12; // support more than one vq
pub const VIRTIO_F_ANY_LAYOUT: u32 = 27;
pub const VIRTIO_RING_F_INDIRECT_DESC: u32 = 28;
pub const VIRTIO_RING_F_EVENT_IDX: u32 = 29;

// this many virtio descriptors.
// must be a power of two.
pub const NUM: usize = 8;

// a single descriptor, from the spec.
#[repr(C)]
pub struct VirtqDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

pub const VRING_DESC_F_NEXT: u16 = 1; // chained with another descriptor
pub const VRING_DESC_F_WRITE: u16 = 2; // device writes (vs read)

// the (entire) avail ring, from the spec.
#[repr(C)]
pub struct VirtqAvail {
    pub flags: u16,       // always zero
    pub idx: u16,         // driver will write ring[idx] next
    pub ring: [u16; NUM], // descriptor numbers of chain heads
    pub unused: u16,
}

// one entry in the "used" ring, with which the
// device tells the driver about completed requests.
pub struct VirtqUsedElem {
    pub id: u32, // index of start of completed descriptor chain
    pub len: u32,
}

pub struct VirtqUsed {
    pub flags: u16, // always zero
    pub idx: u16,   // device increments when it adds a ring[] entry
    pub ring: [VirtqUsedElem; NUM],
}

// these are specific to virtio block devices, e.g. disks,
// described in Section 5.2 of the spec.

pub const VIRTIO_BLK_T_IN: u32 = 0; // read the disk
pub const VIRTIO_BLK_T_OUT: u32 = 1; // write the disk

// the format of the first descriptor in a disk request.
// to be followed by two more descriptors containing
// the block, and a one-byte status.
#[derive(Clone, Copy)]
pub struct VirtioBlqReq {
    pub typ: u32, // VIRTIO_BLK_T_IN or ..._OUT
    pub reserved: u32,
    pub sector: u64,
}
