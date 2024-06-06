pub const MAX_VIRTIO_ID: u64 = 8;

// virtio mmio control registers, mapped starting at 0x10000000 + 0x1000 * id.
pub const VIRTIO_MMIO_BASE: u64 = 0x10001000;
pub enum MmioOffset {
    MagicValue = 0x000,
    Version = 0x004,
    DeviceId = 0x008,
    VendorId = 0x00c,
    DeviceFeatures = 0x010,
    DeviceFeaturesSel = 0x014,
    DriverFeatures = 0x020,
    DriverFeaturesSel = 0x024,
    DriverPageSize = 0x028,
    QueueSel = 0x030,
    QueueNumMax = 0x034,
    QueueNum = 0x038,
    QueueAlign = 0x03c,
    QueuePfn = 0x040,
    QueueReady = 0x044,
    QueueNotify = 0x050,
    InterruptStatus = 0x060,
    InterruptAck = 0x064,
    Status = 0x070,
    QueueDescLow = 0x080,
    QueueDescHigh = 0x084,
    DriverDescLow = 0x090,
    DriverDescHigh = 0x094,
    DeviceDescLow = 0x0a0,
    DeviceDescHigh = 0x0a4,
    Config = 0x100,
}

pub fn virtio_reg(id: u64, reg: MmioOffset) -> &'static mut u32 {
    let addr = VIRTIO_MMIO_BASE + 0x1000 * id + reg as u64;
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
#[repr(C)]
pub struct VirtqUsedElem {
    pub id: u32, // index of start of completed descriptor chain
    pub len: u32,
}

#[repr(C)]
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
#[repr(C)]
pub struct VirtioBlqReq {
    pub typ: u32, // VIRTIO_BLK_T_IN or ..._OUT
    pub reserved: u32,
    pub sector: u64,
}