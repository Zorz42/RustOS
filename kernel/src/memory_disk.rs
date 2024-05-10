use std::{memcpy_non_aligned, Vec};

use crate::disk::Disk;
use crate::memory::{BitSetRaw, DISK_OFFSET, map_page_auto, PAGE_SIZE, VirtAddr};

pub struct MemoryDisk {
    disk: Disk,
    mapped_pages: Vec<i32>,
    bitset: BitSetRaw, // which page is taken
}

fn get_next_page(page: i32) -> i32 {
    let addr = (DISK_OFFSET + page as u64 * PAGE_SIZE + PAGE_SIZE - 4) as *const i32;
    unsafe {
        *addr
    }
}

fn set_next_page(page: i32, next: i32) {
    let addr = (DISK_OFFSET + page as u64 * PAGE_SIZE + PAGE_SIZE - 4) as *mut i32;
    unsafe {
        *addr = next;
    }
}

struct PageIterator {
    addr: *mut u8,
    is_first: bool,
    size_left: i32,
}

impl PageIterator {
    pub fn new(addr: *mut u8) -> Self {
        Self {
            addr,
            is_first: true,
            size_left: unsafe { *(addr as *mut i32) },
        }
    }

    pub fn get_curr_size(&self) -> i32 {
        (if self.is_first {PAGE_SIZE - 8} else {PAGE_SIZE - 4}) as i32
    }

    pub fn get_curr_addr(&self) -> *mut u8 {
        if self.is_first {unsafe { self.addr.add(4) }} else {self.addr}
    }

    pub fn advance(&mut self) -> bool {
        let curr_size = self.get_curr_size();
        if curr_size >= self.size_left {
            return false;
        }

        self.size_left -= curr_size;
        self.addr = (DISK_OFFSET + unsafe { *(self.addr.add(PAGE_SIZE as usize - 4) as *mut i32) } as u64 * PAGE_SIZE) as *mut u8;
        
        true
    }
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
        (self.bitset.get_size_bytes() + PAGE_SIZE as usize - 1) / PAGE_SIZE as usize
    }

    pub fn erase(&mut self) {
        self.bitset.clear();
    }

    fn alloc_page(&mut self) -> i32 {
        let res = self.bitset.get_zero_element();
        if let Some(res) = res {
            self.bitset.set(res, true);
            res as i32
        } else {
            panic!("Out of disk space");
        }
    }

    pub fn create(&mut self) -> i32 {
        let page = self.alloc_page();
        let addr = (DISK_OFFSET + page as u64 * PAGE_SIZE) as *mut i32;
        unsafe {
            *addr = 0;
        }
        page
    }

    pub fn destroy(&mut self, id: i32) {
        todo!();
    }

    pub fn save(&mut self, id: i32, data: &Vec<u8>) {
        todo!();
    }

    pub fn load(&mut self, id: i32) -> Vec<u8> {
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
