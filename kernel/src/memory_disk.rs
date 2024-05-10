use std::{memcpy_non_aligned, Vec};

use crate::disk::Disk;
use crate::memory::{BitSetRaw, DISK_OFFSET, map_page_auto, PAGE_SIZE, VirtAddr};

struct MemoryDisk {
    disk: Disk,
    mapped_pages: Vec<i32>,
    bitset: BitSetRaw, // which page is taken
}

impl MemoryDisk {
    pub fn new(disk: Disk) -> Self {
        let size = disk.size();
        Self {
            disk,
            mapped_pages: Vec::new(),
            bitset: BitSetRaw::new_from(size / 8, (DISK_OFFSET + PAGE_SIZE) as *mut u64),
        }
    }

    pub fn get_num_pages(&self) -> usize {
        self.disk.size() / 4
    }

    pub fn get_size(&self) -> usize {
        self.get_num_pages() * PAGE_SIZE as usize
    }

    fn unmap_page(&self, page: i32) {
        let first_sector = page as u64 * 8;
        for sector in first_sector..first_sector + 8 {
            let mut data = [0; 512];
            unsafe {
                memcpy_non_aligned((DISK_OFFSET + sector * 512) as *mut u8, data.as_mut_ptr(), 512);
            }
            self.disk.write(sector as i32, &data);
        }
    }

    // bitset size in pages
    pub fn get_bitset_size(&self) -> usize {
        (self.get_num_pages() + (PAGE_SIZE as usize * 8) - 1) / (PAGE_SIZE as usize * 8)
    }

    pub fn create(&self) -> i32 {
        todo!();
    }

    pub fn destroy(&self, id: i32) {
        todo!();
    }

    pub fn save(&self, id: i32, data: Vec<u8>) {
        todo!();
    }

    pub fn load(&self, id: i32) -> Vec<u8> {
        todo!();
    }
}

static mut MOUNTED_DISK: Option<MemoryDisk> = None;

pub fn unmount_disk() {
    let mounted_disk = unsafe { &MOUNTED_DISK };

    if let Some(mounted_disk) = mounted_disk {
        for page in &mounted_disk.mapped_pages {
            mounted_disk.unmap_page(*page);
        }

        unsafe {
            MOUNTED_DISK = None;
        }
    }
}

pub fn mount_disk(disk: Disk) {
    unmount_disk();

    let mounted_disk = MemoryDisk::new(disk);
    unsafe {
        MOUNTED_DISK = Some(mounted_disk);
    }
}

pub fn get_mounted_disk() -> &'static mut MemoryDisk {
    unsafe {
        if let Some(mounted_disk) = &mut MOUNTED_DISK {
            mounted_disk
        } else {
            panic!("No disk is mounted.");
        }
    }
}

pub fn disk_page_fault_handler(addr: u64) -> bool {
    if addr < DISK_OFFSET || addr >= DISK_OFFSET + get_mounted_disk().get_size() as u64 {
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
    let first_sector = (addr - DISK_OFFSET) / PAGE_SIZE * 8;
    for sector in first_sector..first_sector + 8 {
        let mut data = mounted_disk.disk.read(sector as i32);
        unsafe {
            memcpy_non_aligned(data.as_mut_ptr(), (DISK_OFFSET + sector * 512) as *mut u8, 512);
        }
    }

    true
}
