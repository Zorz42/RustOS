use std::{memcpy_non_aligned, Vec};

use crate::disk::Disk;
use crate::memory::{DISK_OFFSET, map_page_auto, PAGE_SIZE, VirtAddr};

struct MemoryDisk {
    disk: Disk,
    mapped_pages: Vec<i32>,
}

impl MemoryDisk {
    pub fn new(disk: Disk) -> Self {
        let size = disk.size();
        Self {
            disk,
            mapped_pages: Vec::new_with_size(size / 4),
        }
    }

    pub fn get_num_pages(&self) -> usize {
        self.disk.size() / 4
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

pub fn get_disk_size_bytes() -> usize {
    unsafe {
        if let Some(mounted_disk) = &MOUNTED_DISK {
            mounted_disk.get_num_pages() * PAGE_SIZE as usize
        } else {
            0
        }
    }
}

pub fn disk_page_fault_handler(addr: u64) -> bool {
    if addr < DISK_OFFSET || addr >= DISK_OFFSET + get_disk_size_bytes() as u64 {
        return false;
    }

    let mounted_disk = unsafe {
        if let Some(mounted_disk) = &mut MOUNTED_DISK {
            mounted_disk
        } else {
            return false;
        }
    };

    let idx = (addr - DISK_OFFSET) / PAGE_SIZE;
    mounted_disk.mapped_pages.push(idx as i32);

    let addr = addr / PAGE_SIZE * PAGE_SIZE;
    map_page_auto(addr as VirtAddr, true, false);
    let first_sector = (addr - DISK_OFFSET) / PAGE_SIZE * 4;
    for sector in first_sector..first_sector + 4 {
        let mut data = mounted_disk.disk.read(sector as i32);
        unsafe {
            memcpy_non_aligned(data.as_mut_ptr(), (DISK_OFFSET + sector * 512) as *mut u8, 512);
        }
    }

    true
}
