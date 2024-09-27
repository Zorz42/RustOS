use crate::virtio::definitions::MAX_VIRTIO_ID;
use std::Vec;
use crate::virtio::device::VirtioDevice;

pub const VIRTIO_BLK_F_RO: u32 = 5; // Disk is read-only
pub const VIRTIO_BLK_F_SCSI: u32 = 7; // Supports scsi command passthru
pub const VIRTIO_BLK_F_CONFIG_WCE: u32 = 11; // Writeback mode available in config
pub const VIRTIO_BLK_F_MQ: u32 = 12; // support more than one vq

pub const VIRTIO_BLK_T_IN: u32 = 0; // read the disk
pub const VIRTIO_BLK_T_OUT: u32 = 1; // write the disk

#[derive(Clone, Copy)]
#[repr(C)]
pub struct VirtioBlqReq {
    pub typ: u32,
    pub reserved: u32,
    pub sector: u64,
}

pub struct Disk {
    virtio_device: &'static mut VirtioDevice,
    size: usize,
}

impl Clone for Disk {
    fn clone(&self) -> Self {
        Self {
            virtio_device: unsafe { &mut *(self.virtio_device as *const VirtioDevice as *mut VirtioDevice) },
            size: self.size,
        }
    }
}

pub fn get_disk_at(id: u64) -> Option<Disk> {
    let mut features = !0;
    features &= !(1 << VIRTIO_BLK_F_RO);
    features &= !(1 << VIRTIO_BLK_F_SCSI);
    features &= !(1 << VIRTIO_BLK_F_CONFIG_WCE);
    features &= !(1 << VIRTIO_BLK_F_MQ);

    let device = VirtioDevice::get_device_at(id, 2, 2, 0x554d4551, features);

    if let Some(device) = device {
        let size = unsafe { *(device.get_config_address() as *mut u64) } as usize;

        Some(Disk {
            virtio_device: device,
            size,
        })
    } else {
        None
    }
}

impl Disk {
    pub fn read(&mut self, sector: usize) -> [u8; 512] {
        assert!(sector < self.size);

        let req = VirtioBlqReq {
            typ: VIRTIO_BLK_T_IN,
            reserved: 0,
            sector: sector as u64,
        };

        let res: ([u8; 512], u8) = self.virtio_device.virtio_send_rww(&req);

        assert_eq!(res.1, 0);

        res.0
    }

    pub fn write(&mut self, sector: usize, data: &[u8; 512]) {
        assert!(sector < self.size);

        let req = VirtioBlqReq {
            typ: VIRTIO_BLK_T_OUT,
            reserved: 0,
            sector: sector as u64,
        };

        let res: u8 = self.virtio_device.virtio_send_rrw(&req, data);

        assert_eq!(res, 0);
    }

    pub const fn size(&self) -> usize {
        self.size
    }
}

pub fn scan_for_disks() -> Vec<Disk> {
    let mut vec = Vec::new();
    for id in 0..MAX_VIRTIO_ID {
        if let Some(disk) = get_disk_at(id) {
            vec.push(disk);
        }
    }

    vec
}

