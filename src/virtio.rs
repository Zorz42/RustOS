pub const MAX_VIRTIO_ID: u64 = 8;

pub const VIRTIO_MAGIC: u32 = 0x74726976;

// virtio mmio control registers, mapped starting at 0x10000000 + 0x1000 * id.
#[allow(dead_code)]
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

pub fn virtio_reg_read(id: u64, reg: MmioOffset) -> u32 {
    let addr = VIRTIO_MMIO_BASE + 0x1000 * id + reg as u64;
    unsafe { (addr as *mut u32).read_volatile() }
}

pub fn virtio_reg_write(id: u64, reg: MmioOffset, val: u32) {
    let addr = VIRTIO_MMIO_BASE + 0x1000 * id + reg as u64;
    unsafe { (addr as *mut u32).write_volatile(val); }
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

pub const VIRTIO_GPU_CMD_GET_DISPLAY_INFO: u32 = 0x0100;
pub const VIRTIO_GPU_CMD_RESOURCE_CREATE_2D: u32 = 0x0101;
pub const VIRTIO_GPU_CMD_SET_SCANOUT: u32 = 0x0103;
pub const VIRTIO_GPU_CMD_RESOURCE_FLUSH: u32 = 0x0104;
pub const VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D: u32 = 0x0105;
pub const VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING: u32 = 0x0106;
pub const VIRTIO_GPU_MAX_SCANOUTS: u32 = 16;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct VirtioGpuCtrlHead {
    pub cmd: u32,
    pub flags: u32,
    pub fence_id: u64,
    pub ctx_id: u32,
    pub padding: u32,
}

#[repr(C)]
pub struct VirtioGpuRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
pub struct VirtioGpuDisplayOne {
    pub r: VirtioGpuRect,
    pub enabled: u32,
    pub flags: u32,
}

#[repr(C)]
pub struct VirtioGpuRespDisplayInfo {
    pub hdr: VirtioGpuCtrlHead,
    pub pmodes: [VirtioGpuDisplayOne; VIRTIO_GPU_MAX_SCANOUTS as usize],
}

#[repr(C)]
pub struct VirtioGpuResourceCreate2D {
    pub hdr: VirtioGpuCtrlHead,
    pub resource_id: u32,
    pub format: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
pub struct VirtioGpuResourceAttachBacking {
    pub hdr: VirtioGpuCtrlHead,
    pub resource_id: u32,
    pub nr_entries: u32,
}

#[repr(C)]
pub struct VirtioGpuMemEntry {
    pub addr: u64,
    pub length: u32,
    pub padding: u32,
}

#[repr(C)]
pub struct VirtioGpuSetScanout {
    pub hdr: VirtioGpuCtrlHead,
    pub r: VirtioGpuRect,
    pub scanout_id: u32,
    pub resource_id: u32,
}

#[repr(C)]
pub struct VirtioGpuTransferToHost2D {
    pub hdr: VirtioGpuCtrlHead,
    pub r: VirtioGpuRect,
    pub offset: u64,
    pub resource_id: u32,
    pub padding: u32,
}

#[repr(C)]
pub struct VirtioGpuResourceFlush {
    pub hdr: VirtioGpuCtrlHead,
    pub r: VirtioGpuRect,
    pub resource_id: u32,
    pub padding: u32,
}
