use core::ptr::{copy_nonoverlapping, read_volatile, write_volatile};
use kernel_std::{deserialize, serialize, Serial, Vec, Mutable};

use crate::disk::disk::Disk;
use crate::memory::{map_page_auto, VirtAddr, PAGE_SIZE, unmap_page, BitSet};

pub struct MemoryDisk {
    disk: Disk,
    is_taken: BitSet, // which page is taken
}

impl MemoryDisk {
    pub fn new(disk: &Disk) -> Self {
        let size = disk.size();
        Self {
            disk: disk.clone(),
            is_taken: BitSet::new(0),
        }
    }

    pub fn init(&mut self) {
        self.is_taken = BitSet::new(self.disk.size() / 8);
        let mut data = Vec::new();
        for i in 0..self.get_bitset_num_pages() {
            let page = self.read_page(i as i32 + 1);
            for j in page {
                data.push(j);
            }
        }
        unsafe {
            self.is_taken.load_from(data.as_ptr() as *mut u64);
        }
    }

    pub const fn get_num_pages(&self) -> usize {
        self.disk.size() / 8
    }

    pub fn get_num_free_pages(&self) -> usize {
        self.is_taken.get_count0()
    }

    pub const fn get_size(&self) -> usize {
        self.get_num_pages() * PAGE_SIZE as usize
    }

    // bitset size in pages
    fn get_bitset_num_pages(&self) -> usize {
        self.is_taken.get_size_bytes().div_ceil(PAGE_SIZE as usize)
    }

    pub fn erase(&mut self) {
        self.is_taken.clear();
        for i in 0..=self.get_bitset_num_pages() {
            self.is_taken.set(i, true);
        }

        self.set_head(&Vec::new());
    }

    pub fn read_page(&mut self, page: i32) -> [u8; PAGE_SIZE as usize] {
        let mut data = [0; PAGE_SIZE as usize];
        for i in 0..4 {
            let sector = self.disk.read((page * 4 + i) as usize);
            for j in 0..512 {
                data[(i * 512 + j) as usize] = sector[j as usize];
            }
        }
        data
    }

    pub fn write_page(&mut self, page: i32, data: &[u8; PAGE_SIZE as usize]) {
        for i in 0..4 {
            let mut sector = [0; 512];
            for j in 0..512 {
                sector[j] = data[i * 512 + j];
            }
            self.disk.write(page as usize * 4 + i, &sector);
        }
    }

    pub fn get_head(&mut self) -> Vec<u8> {
        let first_page = self.read_page(0);

        let size = unsafe { *(&first_page[0] as *const u8 as *const i32) } as usize;
        let mut data = Vec::new();

        for i in 0..size {
            data.push(first_page[i + 4]);
        }

        data
    }

    pub fn set_head(&mut self, data: &Vec<u8>) {
        let mut first_page = [0; PAGE_SIZE as usize];

        unsafe {
            write_volatile(&mut first_page[0] as *mut u8 as *mut i32, data.size() as i32);
        }

        for i in 0..data.size() {
            first_page[i + 4] = data.get(i).unwrap().clone();
        }

        self.write_page(0, &first_page);
    }

    pub fn alloc_page(&mut self) -> i32 {
        let res = self.is_taken.get_zero_element();
        if let Some(res) = res {
            self.is_taken.set(res, true);
            res as i32
        } else {
            panic!("Out of disk space");
        }
    }

    pub fn free_page(&mut self, page: i32) {
        debug_assert!(self.is_taken.get(page as usize));
        self.is_taken.set(page as usize, false);
    }
}

static MOUNTED_DISK: Mutable<Option<MemoryDisk>> = Mutable::new(None);

pub fn unmount_disk() {
    let t = MOUNTED_DISK.borrow();
    if let Some(mounted_disk) = MOUNTED_DISK.get_mut(&t) {
        let num_pages = mounted_disk.get_bitset_num_pages();
        let mut data = unsafe { Vec::new_with_size_uninit(num_pages * PAGE_SIZE as usize) };
        unsafe {
            mounted_disk.is_taken.store_to(data.as_mut_ptr() as *mut u64);
        }


        for i in 0..num_pages {
            let mut page = [0; PAGE_SIZE as usize];
            for j in 0..PAGE_SIZE as usize {
                if i * PAGE_SIZE as usize + j < data.size() {
                    page[j] = data[i * PAGE_SIZE as usize + j];
                }
            }
            mounted_disk.write_page(i as i32 + 1, &page);
        }
    }

    *MOUNTED_DISK.get_mut(&t) = None;
    MOUNTED_DISK.release(t);
}

pub fn mount_disk(disk: &Disk) {
    unmount_disk();

    let t = MOUNTED_DISK.borrow();
    let mounted_disk = MemoryDisk::new(disk);
    *MOUNTED_DISK.get_mut(&t) = Some(mounted_disk);
    MOUNTED_DISK.get_mut(&t).as_mut().unwrap().init();
    MOUNTED_DISK.release(t);
}

pub fn get_mounted_disk() -> &'static Mutable<Option<MemoryDisk>> {
    &MOUNTED_DISK
}