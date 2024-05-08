use std::Vec;

use crate::disk::Disk;

struct MemoryDisk {
    disk: Disk,
    mapped_pages: Vec<u64>, // a bitset, 1 if page is mapped
}

impl MemoryDisk {
    pub fn new(disk: Disk) -> Self {
        let size = disk.size();
        Self {
            disk,
            mapped_pages: Vec::new_with_size((size / 4 + 63) / 64),
        }
    }

    pub fn get_num_pages(&self) {
        self.disk.size() / 4;
    }
}

static mut MOUNTED_DISK: Option<MemoryDisk> = None;

pub fn unmount_disk() {
    let mounted_disk = unsafe { &MOUNTED_DISK };

    if let Some(mounted_disk) = mounted_disk {
        unsafe {
            MOUNTED_DISK = None;
        }
    }
}

pub fn mount_disk(disk: Disk) {
    let mounted_disk = MemoryDisk::new(disk);
    unsafe {
        MOUNTED_DISK = Some(mounted_disk);
    }
}
