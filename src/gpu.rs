use core::arch::asm;
use core::mem::size_of;
use core::ptr::{addr_of, write_bytes};
use core::sync::atomic::{fence, Ordering};
use crate::spinlock::Lock;
use crate::virtio::definitions::{virtio_reg_read, VirtqAvail, VirtqDesc, VirtqUsed, MmioOffset, NUM, VIRTIO_CONFIG_S_ACKNOWLEDGE, VIRTIO_CONFIG_S_DRIVER, VIRTIO_F_ANY_LAYOUT, VIRTIO_RING_F_EVENT_IDX, VIRTIO_RING_F_INDIRECT_DESC, VIRTIO_CONFIG_S_FEATURES_OK, VIRTIO_CONFIG_S_DRIVER_OK, VRING_DESC_F_NEXT, VRING_DESC_F_WRITE, MAX_VIRTIO_ID, virtio_reg_write, VIRTIO_MAGIC};
use crate::memory::{alloc_continuous_pages, alloc_page, PAGE_SIZE};
use crate::panic;
use crate::riscv::get_core_id;
use crate::timer::get_ticks;

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

#[derive(Clone, Copy)]
struct Info {
    ready: u8,
    status: u8,
}

struct Gpu {
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
    ops: [VirtioGpuCtrlHead; NUM],

    vgpu_lock: Lock,

    id: u64,

    irq_waiting: bool, // if an irq was cancelled because it was locked. do it when it unlocks

    pixels_size: (u32, u32),

    framebuffer: *mut u32,
}

fn get_gpu_at(id: u64) -> Option<&'static mut Gpu> {
    if virtio_reg_read(id, MmioOffset::MagicValue) != VIRTIO_MAGIC
        || virtio_reg_read(id, MmioOffset::Version) != 2
        || virtio_reg_read(id, MmioOffset::DeviceId) != 16
        || virtio_reg_read(id, MmioOffset::VendorId) != 0x554d4551
    {
        return None;
    }

    let gpu = unsafe {&mut *(alloc_page() as *mut Gpu) };

    *gpu = Gpu {
        desc: 0 as *mut VirtqDesc,
        avail: 0 as *mut VirtqAvail,
        used: 0 as *mut VirtqUsed,
        free: [true; NUM],
        used_idx: 0,
        info: [Info { ready: 0, status: 0 }; NUM],
        ops: [VirtioGpuCtrlHead {
            cmd: 0,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            padding: 0,
        }; NUM],
        vgpu_lock: Lock::new(),
        id,
        irq_waiting: false,
        pixels_size: (0, 0),
        framebuffer: 0 as *mut u32,
    };

    let mut status = 0;

    status |= VIRTIO_CONFIG_S_ACKNOWLEDGE;
    virtio_reg_write(id, MmioOffset::Status, status);

    status |= VIRTIO_CONFIG_S_DRIVER;
    virtio_reg_write(id, MmioOffset::Status, status);

    // negotiate features
    let mut features = virtio_reg_read(id, MmioOffset::DeviceFeatures);
    features &= !(1 << VIRTIO_F_ANY_LAYOUT);
    features &= !(1 << VIRTIO_RING_F_EVENT_IDX);
    features &= !(1 << VIRTIO_RING_F_INDIRECT_DESC);
    virtio_reg_write(id, MmioOffset::DriverFeatures, features);

    status |= VIRTIO_CONFIG_S_FEATURES_OK;
    virtio_reg_write(id, MmioOffset::Status, status);

    // reread and check
    status = virtio_reg_read(id, MmioOffset::Status);
    assert_eq!(status & VIRTIO_CONFIG_S_FEATURES_OK, VIRTIO_CONFIG_S_FEATURES_OK);

    virtio_reg_write(id, MmioOffset::QueueSel, 0);

    assert_eq!(virtio_reg_read(id, MmioOffset::QueueReady), 0);

    assert!(virtio_reg_read(id, MmioOffset::QueueNumMax) >= NUM as u32);

    gpu.desc = alloc_page() as *mut VirtqDesc;
    gpu.avail = alloc_page() as *mut VirtqAvail;
    gpu.used = alloc_page() as *mut VirtqUsed;

    unsafe {
        write_bytes(gpu.desc as *mut u8, 0, PAGE_SIZE as usize);
        write_bytes(gpu.avail as *mut u8, 0, PAGE_SIZE as usize);
        write_bytes(gpu.used as *mut u8, 0, PAGE_SIZE as usize);
    }

    virtio_reg_write(id, MmioOffset::QueueNum, NUM as u32);

    virtio_reg_write(id, MmioOffset::QueueDescLow, (gpu.desc as u64 & 0xFFFFFFFF) as u32);
    virtio_reg_write(id, MmioOffset::QueueDescHigh, ((gpu.desc as u64 >> 32) & 0xFFFFFFFF) as u32);
    virtio_reg_write(id, MmioOffset::DriverDescLow, (gpu.avail as u64 & 0xFFFFFFFF) as u32);
    virtio_reg_write(id, MmioOffset::DriverDescHigh, ((gpu.avail as u64 >> 32) & 0xFFFFFFFF) as u32);
    virtio_reg_write(id, MmioOffset::DeviceDescLow, (gpu.used as u64 & 0xFFFFFFFF) as u32);
    virtio_reg_write(id, MmioOffset::DeviceDescHigh, ((gpu.used as u64 >> 32) & 0xFFFFFFFF) as u32);

    // queue is ready
    virtio_reg_write(id, MmioOffset::QueueReady, 1);

    // we are completely ready
    status |= VIRTIO_CONFIG_S_DRIVER_OK;
    virtio_reg_write(id, MmioOffset::Status, status);

    Some(gpu)
}

impl Gpu {
    fn alloc_desc(&mut self) -> Option<usize> {
        for i in 0..NUM {
            if self.free[i] {
                self.free[i] = false;
                return Some(i);
            }
        }
        None
    }

    fn get_desc(&mut self, idx: usize) -> &mut VirtqDesc {
        assert!(idx < NUM);
        unsafe { &mut *self.desc.add(idx) }
    }

    fn free_desc(&mut self, idx: usize) {
        assert!(!self.free[idx]);

        let desc = self.get_desc(idx);
        desc.next = 0;
        desc.flags = 0;
        desc.next = 0;
        desc.len = 0;
        self.free[idx] = true;
    }

    fn alloc_3desc(&mut self) -> Option<[usize; 3]> {
        let mut res = [0; 3];

        for (i, r) in res.iter_mut().enumerate() {
            let desc = self.alloc_desc();
            if let Some(desc) = desc {
                *r = desc;
            } else {
                for j in 0..i {
                    self.free_desc(j);
                }
                return None;
            }
        }

        Some(res)
    }

    fn alloc_2desc(&mut self) -> Option<[usize; 2]> {
        let mut res = [0; 2];

        for (i, r) in res.iter_mut().enumerate() {
            let desc = self.alloc_desc();
            if let Some(desc) = desc {
                *r = desc;
            } else {
                for j in 0..i {
                    self.free_desc(j);
                }
                return None;
            }
        }

        Some(res)
    }

    fn free_chain(&mut self, mut idx: usize) {
        loop {
            let flags = self.get_desc(idx).flags;
            let next = self.get_desc(idx).next as usize;
            self.free_desc(idx);
            if (flags & VRING_DESC_F_NEXT) == 0 {
                break;
            }
            idx = next;
        }
    }

    fn virtio_fetch_resolution(&mut self) {
        self.vgpu_lock.spinlock();

        let idx = self.alloc_3desc().unwrap();

        let buf0 = unsafe { &mut *((&mut self.ops[idx[0]]) as *mut VirtioGpuCtrlHead) };
        buf0.cmd = VIRTIO_GPU_CMD_GET_DISPLAY_INFO;

        let desc0 = self.get_desc(idx[0]);
        desc0.addr = addr_of!(*buf0) as u64;
        desc0.len = size_of::<VirtioGpuCtrlHead>() as u32;
        desc0.flags = VRING_DESC_F_NEXT;
        desc0.next = idx[1] as u16;

        let page = alloc_page();

        let desc1 = self.get_desc(idx[1]);
        desc1.addr = page;
        desc1.len = size_of::<VirtioGpuRespDisplayInfo>() as u32;
        desc1.flags = VRING_DESC_F_WRITE;
        desc1.next = 0;

        self.info[idx[0]].ready = 0;

        unsafe {
            let idx2 = (*self.avail).idx as usize;
            (*self.avail).ring[idx2 % NUM] = idx[0] as u16;
        }

        fence(Ordering::Release);

        unsafe {
            (*self.avail).idx += 1;
        }

        fence(Ordering::Release);

        virtio_reg_write(self.id, MmioOffset::QueueNotify, 0);

        self.vgpu_lock.unlock();
        if self.irq_waiting {
            gpu_irq(self.id as u32 + 1);
        }
        while self.info[idx[0]].ready == 0 {
            unsafe {
                asm!("wfi");
            }
        }
        self.vgpu_lock.spinlock();

        self.free_chain(idx[0]);

        self.vgpu_lock.unlock();

        if self.irq_waiting {
            gpu_irq(self.id as u32 + 1);
        }

        let response = unsafe { &*(page as *const VirtioGpuRespDisplayInfo) };

        let rect = &response.pmodes[0].r;
        self.pixels_size = (rect.width, rect.height);
    }

    fn virtio_create_resource(&mut self) {
        self.vgpu_lock.spinlock();

        let idx = self.alloc_2desc().unwrap();

        let req = VirtioGpuResourceCreate2D {
            hdr: VirtioGpuCtrlHead {
                cmd: VIRTIO_GPU_CMD_RESOURCE_CREATE_2D,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            },
            resource_id: 1,
            format: 67,
            width: self.pixels_size.0,
            height: self.pixels_size.1,
        };

        let desc0 = self.get_desc(idx[0]);
        desc0.addr = addr_of!(req) as u64;
        desc0.len = size_of::<VirtioGpuResourceCreate2D>() as u32;
        desc0.flags = VRING_DESC_F_NEXT;
        desc0.next = idx[1] as u16;

        let page = alloc_page();

        let desc1 = self.get_desc(idx[1]);
        desc1.addr = page;
        desc1.len = size_of::<VirtioGpuCtrlHead>() as u32;
        desc1.flags = VRING_DESC_F_WRITE;
        desc1.next = 0;

        self.info[idx[0]].ready = 0;

        unsafe {
            let idx2 = (*self.avail).idx as usize;
            (*self.avail).ring[idx2 % NUM] = idx[0] as u16;
        }

        fence(Ordering::Release);

        unsafe {
            (*self.avail).idx += 1;
        }

        fence(Ordering::Release);

        virtio_reg_write(self.id, MmioOffset::QueueNotify, 0);

        self.vgpu_lock.unlock();
        if self.irq_waiting {
            gpu_irq(self.id as u32 + 1);
        }
        while self.info[idx[0]].ready == 0 {
            unsafe {
                asm!("wfi");
            }
        }
        self.vgpu_lock.spinlock();

        self.free_chain(idx[0]);

        self.vgpu_lock.unlock();

        if self.irq_waiting {
            gpu_irq(self.id as u32 + 1);
        }
    }

    fn virtio_create_framebuffer(&mut self) {
        self.vgpu_lock.spinlock();

        let idx = self.alloc_3desc().unwrap();

        let req = VirtioGpuResourceAttachBacking {
            hdr: VirtioGpuCtrlHead {
                cmd: VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            },
            resource_id: 1,
            nr_entries: 1,
        };

        let framebuffer_size = (self.pixels_size.0 * self.pixels_size.1 * 4) as u64;
        let num_pages = (framebuffer_size + PAGE_SIZE - 1) / PAGE_SIZE;

        self.framebuffer = alloc_continuous_pages(num_pages) as *mut u32;

        let req2 = VirtioGpuMemEntry {
            addr: self.framebuffer as u64,
            length: framebuffer_size as u32,
            padding: 0,
        };

        let desc0 = self.get_desc(idx[0]);
        desc0.addr = addr_of!(req) as u64;
        desc0.len = size_of::<VirtioGpuResourceAttachBacking>() as u32;
        desc0.flags = VRING_DESC_F_NEXT;
        desc0.next = idx[1] as u16;

        let desc1 = self.get_desc(idx[1]);
        desc1.addr = addr_of!(req2) as u64;
        desc1.len = size_of::<VirtioGpuMemEntry>() as u32;
        desc1.flags = VRING_DESC_F_NEXT;
        desc1.next = idx[2] as u16;

        let page = alloc_page();

        let desc2 = self.get_desc(idx[2]);
        desc2.addr = page;
        desc2.len = size_of::<VirtioGpuCtrlHead>() as u32;
        desc2.flags = VRING_DESC_F_WRITE;
        desc2.next = 0;

        self.info[idx[0]].ready = 0;

        unsafe {
            let idx2 = (*self.avail).idx as usize;
            (*self.avail).ring[idx2 % NUM] = idx[0] as u16;
        }

        fence(Ordering::Release);

        unsafe {
            (*self.avail).idx += 1;
        }

        fence(Ordering::Release);

        virtio_reg_write(self.id, MmioOffset::QueueNotify, 0);

        self.vgpu_lock.unlock();
        if self.irq_waiting {
            gpu_irq(self.id as u32 + 1);
        }
        while self.info[idx[0]].ready == 0 {
            unsafe {
                asm!("wfi");
            }
        }
        self.vgpu_lock.spinlock();

        self.free_chain(idx[0]);

        self.vgpu_lock.unlock();

        if self.irq_waiting {
            gpu_irq(self.id as u32 + 1);
        }
    }

    fn virtio_set_scanout(&mut self) {
        self.vgpu_lock.spinlock();

        let idx = self.alloc_2desc().unwrap();

        let req = VirtioGpuSetScanout {
            hdr: VirtioGpuCtrlHead {
                cmd: VIRTIO_GPU_CMD_SET_SCANOUT,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            },
            r: VirtioGpuRect {
                x: 0,
                y: 0,
                width: self.pixels_size.0,
                height: self.pixels_size.1,
            },
            scanout_id: 0,
            resource_id: 1,
        };

        let desc0 = self.get_desc(idx[0]);
        desc0.addr = addr_of!(req) as u64;
        desc0.len = size_of::<VirtioGpuSetScanout>() as u32;
        desc0.flags = VRING_DESC_F_NEXT;
        desc0.next = idx[1] as u16;

        let page = alloc_page();

        let desc1 = self.get_desc(idx[1]);
        desc1.addr = page;
        desc1.len = size_of::<VirtioGpuCtrlHead>() as u32;
        desc1.flags = VRING_DESC_F_WRITE;
        desc1.next = 0;

        self.info[idx[0]].ready = 0;

        unsafe {
            let idx2 = (*self.avail).idx as usize;
            (*self.avail).ring[idx2 % NUM] = idx[0] as u16;
        }

        fence(Ordering::Release);

        unsafe {
            (*self.avail).idx += 1;
        }

        fence(Ordering::Release);

        virtio_reg_write(self.id, MmioOffset::QueueNotify, 0);

        self.vgpu_lock.unlock();
        if self.irq_waiting {
            gpu_irq(self.id as u32 + 1);
        }
        while self.info[idx[0]].ready == 0 {
            unsafe {
                asm!("wfi");
            }
        }
        self.vgpu_lock.spinlock();

        self.free_chain(idx[0]);

        self.vgpu_lock.unlock();

        if self.irq_waiting {
            gpu_irq(self.id as u32 + 1);
        }
    }

    fn virtio_transfer_to_host(&mut self) {
        self.vgpu_lock.spinlock();

        let idx = self.alloc_2desc().unwrap();

        let req = VirtioGpuTransferToHost2D {
            hdr: VirtioGpuCtrlHead {
                cmd: VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            },
            r: VirtioGpuRect {
                x: 0,
                y: 0,
                width: self.pixels_size.0,
                height: self.pixels_size.1,
            },
            offset: 0,
            resource_id: 1,
            padding: 0,
        };

        let desc0 = self.get_desc(idx[0]);
        desc0.addr = addr_of!(req) as u64;
        desc0.len = size_of::<VirtioGpuTransferToHost2D>() as u32;
        desc0.flags = VRING_DESC_F_NEXT;
        desc0.next = idx[1] as u16;

        let page = alloc_page();

        let desc1 = self.get_desc(idx[1]);
        desc1.addr = page;
        desc1.len = size_of::<VirtioGpuCtrlHead>() as u32;
        desc1.flags = VRING_DESC_F_WRITE;
        desc1.next = 0;

        self.info[idx[0]].ready = 0;

        unsafe {
            let idx2 = (*self.avail).idx as usize;
            (*self.avail).ring[idx2 % NUM] = idx[0] as u16;
        }

        fence(Ordering::Release);

        unsafe {
            (*self.avail).idx += 1;
        }

        fence(Ordering::Release);

        virtio_reg_write(self.id, MmioOffset::QueueNotify, 0);

        self.vgpu_lock.unlock();
        if self.irq_waiting {
            gpu_irq(self.id as u32 + 1);
        }

        let start_time = get_ticks();
        while self.info[idx[0]].ready == 0 {
            unsafe {
                asm!("wfi");
            }

            if get_ticks() - start_time > 1000 {
                panic!("GPU transfer to host timeout");
            }
        }
        self.vgpu_lock.spinlock();

        self.free_chain(idx[0]);

        self.vgpu_lock.unlock();

        if self.irq_waiting {
            gpu_irq(self.id as u32 + 1);
        }
    }

    fn virtio_flush_resource(&mut self) {
        self.vgpu_lock.spinlock();

        let idx = self.alloc_2desc().unwrap();

        let req = VirtioGpuResourceFlush {
            hdr: VirtioGpuCtrlHead {
                cmd: VIRTIO_GPU_CMD_RESOURCE_FLUSH,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            },
            r: VirtioGpuRect {
                x: 0,
                y: 0,
                width: self.pixels_size.0,
                height: self.pixels_size.1,
            },
            resource_id: 1,
            padding: 0,
        };

        let desc0 = self.get_desc(idx[0]);
        desc0.addr = addr_of!(req) as u64;
        desc0.len = size_of::<VirtioGpuResourceFlush>() as u32;
        desc0.flags = VRING_DESC_F_NEXT;
        desc0.next = idx[1] as u16;

        let page = alloc_page();

        let desc1 = self.get_desc(idx[1]);
        desc1.addr = page;
        desc1.len = size_of::<VirtioGpuCtrlHead>() as u32;
        desc1.flags = VRING_DESC_F_WRITE;
        desc1.next = 0;

        self.info[idx[0]].ready = 0;

        unsafe {
            let idx2 = (*self.avail).idx as usize;
            (*self.avail).ring[idx2 % NUM] = idx[0] as u16;
        }

        fence(Ordering::Release);

        unsafe {
            (*self.avail).idx += 1;
        }

        fence(Ordering::Release);

        virtio_reg_write(self.id, MmioOffset::QueueNotify, 0);

        self.vgpu_lock.unlock();
        if self.irq_waiting {
            gpu_irq(self.id as u32 + 1);
        }

        let start_time = get_ticks();
        while self.info[idx[0]].ready == 0 {
            unsafe {
                asm!("wfi");
            }

            if get_ticks() - start_time > 1000 {
                panic!("GPU flush resource timeout");
            }
        }
        self.vgpu_lock.spinlock();

        self.free_chain(idx[0]);

        self.vgpu_lock.unlock();

        if self.irq_waiting {
            gpu_irq(self.id as u32 + 1);
        }
    }

    fn refresh_screen(&mut self) {
        //self.virtio_transfer_to_host();
        //self.virtio_flush_resource();
    }
}

static mut GPU: Option<&'static mut Gpu> = None;
static mut GPU_ID: u64 = 0;

pub fn init_gpu() {
    for id in 0..MAX_VIRTIO_ID {
        if let Some(gpu) = get_gpu_at(id) {
            unsafe {
                GPU = Some(gpu);
                GPU_ID = id;
                GPU.as_mut().unwrap().virtio_fetch_resolution();
                GPU.as_mut().unwrap().virtio_create_resource();
                GPU.as_mut().unwrap().virtio_create_framebuffer();
                GPU.as_mut().unwrap().virtio_set_scanout();
            }
            break;
        }
    }
}

pub fn get_framebuffer() -> *mut u32 {
    unsafe { GPU.as_ref().unwrap().framebuffer }
}

pub fn refresh_screen() {
    unsafe { GPU.as_mut().unwrap().refresh_screen() }
}

pub fn get_screen_size() -> (u32, u32) {
    unsafe { GPU.as_ref().unwrap().pixels_size }
}

pub fn gpu_irq(irq: u32) {
    if irq == 0 || irq > MAX_VIRTIO_ID as u32 {
        return;
    }
    let id = irq as u64 - 1;

    if id != unsafe { GPU_ID } {
        return;
    }

    let gpu = unsafe { GPU.as_mut().unwrap() };

    if gpu.vgpu_lock.locked_by() == get_core_id() as i32 {
        gpu.irq_waiting = true;
        return;
    }
    gpu.vgpu_lock.spinlock();

    virtio_reg_write(id, MmioOffset::InterruptAck, virtio_reg_read(id, MmioOffset::InterruptStatus) & 0x3);

    fence(Ordering::Release);

    let used = unsafe { &mut *gpu.used };

    while gpu.used_idx != used.idx {
        fence(Ordering::Release);
        let id = used.ring[gpu.used_idx as usize % NUM].id;

        gpu.info[id as usize].ready = 1;

        gpu.used_idx += 1;
    }

    gpu.vgpu_lock.unlock();
}
