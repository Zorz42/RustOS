use core::ptr::{copy_nonoverlapping, read_volatile, write_volatile};
use kernel_std::{deserialize, serialize, Serial, Vec, Mutable};

use crate::disk::disk::Disk;
use crate::memory::{map_page_auto, VirtAddr, DISK_OFFSET, PAGE_SIZE, unmap_page, BitSet};

pub struct MemoryDisk {
    disk: Disk,
    mapped_pages: Vec<i32>,
    is_taken: BitSet, // which page is taken
    is_mapped: BitSet, // which page is mapped
    in_vec: BitSet, // which page is in vector mapped_pages
}

const fn id_to_addr(page: i32) -> *mut u8 {
    (DISK_OFFSET + page as u64 * PAGE_SIZE) as *mut u8
}

impl MemoryDisk {
    pub fn new(disk: &Disk) -> Self {
        let size = disk.size();
        Self {
            disk: disk.clone(),
            mapped_pages: Vec::new(),
            is_taken: BitSet::new(0),
            is_mapped: BitSet::new(size / 8),
            in_vec: BitSet::new(size / 8),
        }
    }

    pub fn init(&mut self) {
        self.is_taken = BitSet::new(self.disk.size() / 8);
        self.declare_read(id_to_addr(1) as u64, id_to_addr(1) as u64 + self.get_bitset_num_pages() as u64 * PAGE_SIZE);
        unsafe {
            self.is_taken.load_from(id_to_addr(1) as *mut u64);
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

    fn map_page(&mut self, addr: u64, load: bool) {
        let idx = (addr - DISK_OFFSET) / PAGE_SIZE;
        if self.is_mapped.get(idx as usize) {
            return;
        }
        self.is_mapped.set(idx as usize, true);
        if !self.in_vec.get(idx as usize) {
            self.mapped_pages.push(idx as i32);
        }
        self.in_vec.set(idx as usize, true);

        let addr = addr / PAGE_SIZE * PAGE_SIZE;
        map_page_auto(addr as VirtAddr, false, true, false, false);
        if load {
            let first_sector = (addr - DISK_OFFSET) / PAGE_SIZE * 8;
            for sector in first_sector..first_sector + 8 {
                let mut data = self.disk.read(sector as usize);
                unsafe {
                    copy_nonoverlapping(data.as_mut_ptr(), (DISK_OFFSET + sector * 512) as *mut u8, 512);
                }
            }
        }
    }

    fn map_range(&mut self, low_addr: u64, high_addr: u64, load: bool) {
        debug_assert!(low_addr <= high_addr);
        debug_assert!(DISK_OFFSET <= low_addr);
        debug_assert!(high_addr <= DISK_OFFSET + PAGE_SIZE * self.get_num_pages() as u64);

        let low_page = low_addr / PAGE_SIZE;
        let high_page = high_addr.div_ceil(PAGE_SIZE);

        for page in low_page..high_page {
            let page_addr = page * PAGE_SIZE;
            self.map_page(page_addr, load);
        }
    }

    pub fn declare_write(&mut self, low_addr: u64, high_addr: u64) {
        self.map_range(low_addr, high_addr, false);
    }

    pub fn declare_read(&mut self, low_addr: u64, high_addr: u64) {
        self.map_range(low_addr, high_addr, true);
    }

    fn unmap_page(&mut self, page: i32) {
        if !self.is_mapped.get(page as usize) {
            return;
        }
        self.is_mapped.set(page as usize, false);

        let first_sector = page as u64 * 8;
        for sector in first_sector..first_sector + 8 {
            let mut data = [0; 512];
            unsafe {
                copy_nonoverlapping((DISK_OFFSET + sector * 512) as *mut u8, data.as_mut_ptr(), 512);
            }
            self.disk.write(sector as usize, &data);
        }
        unmap_page(id_to_addr(page));
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

    pub fn get_head(&mut self) -> Vec<u8> {
        self.declare_read(DISK_OFFSET, DISK_OFFSET + 4);

        let size = unsafe { *(DISK_OFFSET as *mut i32) } as usize;
        let mut data = Vec::new();

        self.declare_read(DISK_OFFSET + 4, DISK_OFFSET + 4 + size as u64);
        let ptr = (DISK_OFFSET + 4) as *mut u8;
        for i in 0..size {
            data.push(unsafe { read_volatile(ptr.add(i)) });
        }

        data
    }

    pub fn set_head(&mut self, data: &Vec<u8>) {
        self.declare_write(DISK_OFFSET, DISK_OFFSET + 4 + data.size() as u64);

        unsafe {
            write_volatile(DISK_OFFSET as *mut i32, data.size() as i32);
        }

        let mut ptr = (DISK_OFFSET + 4) as *mut u8;
        for i in data {
            unsafe {
                write_volatile(ptr, *i);
                ptr = ptr.add(1);
            }
        }
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
        let temp = mounted_disk.get_bitset_num_pages();
        mounted_disk.declare_write(id_to_addr(1) as u64, id_to_addr(1) as u64 + temp as u64 * PAGE_SIZE);
        unsafe {
            mounted_disk.is_taken.store_to(id_to_addr(1) as *mut u64);
        }

        for page in mounted_disk.mapped_pages.clone() {
            mounted_disk.unmap_page(page);
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