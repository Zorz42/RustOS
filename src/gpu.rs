use std::println;
use crate::memory::{alloc_continuous_pages, PAGE_SIZE};
use crate::virtio::definitions::MAX_VIRTIO_ID;
use crate::virtio::device::VirtioDevice;

const VIRTIO_GPU_CMD_GET_DISPLAY_INFO: u32 = 0x0100;
const VIRTIO_GPU_CMD_RESOURCE_CREATE_2D: u32 = 0x0101;
const VIRTIO_GPU_CMD_SET_SCANOUT: u32 = 0x0103;
const VIRTIO_GPU_CMD_RESOURCE_FLUSH: u32 = 0x0104;
const VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D: u32 = 0x0105;
const VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING: u32 = 0x0106;
const VIRTIO_GPU_MAX_SCANOUTS: u32 = 16;
const VIRTIO_GPU_RESP_OK_NODATA: u32 = 0x1100;
const VIRTIO_GPU_RESP_OK_DISPLAY_INFO: u32 = 0x1101;

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
    virtio_device: &'static mut VirtioDevice,

    pixels_size: (u32, u32),

    framebuffer: *mut u32,
}

fn get_gpu_at(id: u64) -> Option<Gpu> {
    let device = VirtioDevice::get_device_at(id, 2, 2, 0x554d4551, !0);

    if let Some(device) = device {
        Some(Gpu {
            virtio_device: device,
            pixels_size: (0, 0),
            framebuffer: 0 as *mut u32,
        })
    } else {
        None
    }


    /*if virtio_reg_read(id, MmioOffset::MagicValue) != VIRTIO_MAGIC
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

    Some(gpu)*/
}

impl Gpu {
    fn virtio_fetch_resolution(&mut self) {
        let req = VirtioGpuCtrlHead {
            cmd: VIRTIO_GPU_CMD_GET_DISPLAY_INFO,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            padding: 0,
        };

        let resp: VirtioGpuRespDisplayInfo = self.virtio_device.virtio_send_rw(&req);

        //assert_eq!(resp.hdr.cmd, VIRTIO_GPU_RESP_OK_DISPLAY_INFO);

        let rect = &resp.pmodes[0].r;
        self.pixels_size = (rect.width, rect.height);

        println!("Resolution: {}x{}", self.pixels_size.0, self.pixels_size.1);

        println!("looping...");
        loop {}

        /*self.vgpu_lock.spinlock();

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
        self.pixels_size = (rect.width, rect.height);*/
    }

    fn virtio_create_resource(&mut self) {
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

        let resp: VirtioGpuCtrlHead = self.virtio_device.virtio_send_rw(&req);
        assert_eq!(resp.cmd, VIRTIO_GPU_RESP_OK_NODATA);

        /*self.vgpu_lock.spinlock();

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
        }*/
    }

    fn virtio_create_framebuffer(&mut self) {
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

        let resp: VirtioGpuCtrlHead = self.virtio_device.virtio_send_rrw(&req, &req2);
        assert_eq!(resp.cmd, VIRTIO_GPU_RESP_OK_NODATA);

        /*self.vgpu_lock.spinlock();

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
        }*/
    }

    fn virtio_set_scanout(&mut self) {
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

        let resp: VirtioGpuCtrlHead = self.virtio_device.virtio_send_rw(&req);
        assert_eq!(resp.cmd, VIRTIO_GPU_RESP_OK_NODATA);

        /*self.vgpu_lock.spinlock();

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
        }*/
    }

    fn virtio_transfer_to_host(&mut self) {
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

        let resp: VirtioGpuCtrlHead = self.virtio_device.virtio_send_rw(&req);
        assert_eq!(resp.cmd, VIRTIO_GPU_RESP_OK_NODATA);

        /*self.vgpu_lock.spinlock();

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
        }*/
    }

    fn virtio_flush_resource(&mut self) {
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

        let resp: VirtioGpuCtrlHead = self.virtio_device.virtio_send_rw(&req);
        assert_eq!(resp.cmd, VIRTIO_GPU_RESP_OK_NODATA);

        /*self.vgpu_lock.spinlock();

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
        }*/
    }

    fn refresh_screen(&mut self) {
        self.virtio_transfer_to_host();
        self.virtio_flush_resource();
    }
}

static mut GPU: Option<Gpu> = None;

pub fn init_gpu() {
    for id in 0..MAX_VIRTIO_ID {
        if let Some(gpu) = get_gpu_at(id) {
            unsafe {
                GPU = Some(gpu);
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

/*pub fn gpu_irq(irq: u32) {
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
}*/
