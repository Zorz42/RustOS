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

struct Gpu {
    virtio_device: &'static mut VirtioDevice,
    pixels_size: (u32, u32),
    framebuffer: *mut u32,
}

fn get_gpu_at(id: u64) -> Option<Gpu> {
    let device = VirtioDevice::get_device_at(id, 2, 16, 0x554d4551, !0);

    device.map(|device| Gpu {
        virtio_device: device,
        pixels_size: (0, 0),
        framebuffer: 0 as *mut u32,
    })
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

        assert_eq!(resp.hdr.cmd, VIRTIO_GPU_RESP_OK_DISPLAY_INFO);

        let rect = &resp.pmodes[0].r;
        self.pixels_size = (rect.width, rect.height);
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
