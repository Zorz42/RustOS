use core::ptr::copy_nonoverlapping;
use std::{deserialize, serialize, Serial, Vec, Box};

use crate::disk::disk::Disk;
use crate::memory::{map_page_auto, VirtAddr, DISK_OFFSET, PAGE_SIZE, unmap_page, BitSet};

pub struct MemoryDisk {
    disk: &'static mut Disk,
    mapped_pages: Vec<i32>,
    is_taken: Option<BitSet>, // which page is taken
    is_mapped: BitSet, // which page is mapped
    in_vec: BitSet, // which page is in vector
}

fn id_to_addr(page: i32) -> *mut u8 {
    (DISK_OFFSET + page as u64 * PAGE_SIZE) as *mut u8
}

impl MemoryDisk {
    pub fn new(disk: &'static mut Disk) -> Self {
        let size = disk.size();
        Self {
            disk,
            mapped_pages: Vec::new(),
            is_taken: None,
            is_mapped: BitSet::new(size / 8),
            in_vec: BitSet::new(size / 8),
        }
    }

    pub fn init(&mut self) {
        self.is_taken = Some(BitSet::new(self.disk.size() / 8));
        self.declare_read(id_to_addr(1) as u64, id_to_addr(1) as u64 + self.get_bitset_num_pages() as u64 * PAGE_SIZE);
        unsafe {
            self.is_taken.as_mut().unwrap().load_from(id_to_addr(1) as *mut u64);
        }
    }

    fn get_is_taken(&self) -> &BitSet {
        self.is_taken.as_ref().unwrap()
    }

    fn get_is_taken_mut(&mut self) -> &mut BitSet {
        self.is_taken.as_mut().unwrap()
    }

    pub fn get_num_pages(&self) -> usize {
        self.disk.size() / 8
    }

    pub fn get_num_free_pages(&self) -> usize {
        self.get_is_taken().get_count0()
    }

    pub fn get_size(&self) -> usize {
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
        map_page_auto(addr as VirtAddr, false, true, false);
        if load {
            let first_sector = (addr - DISK_OFFSET) / PAGE_SIZE * 8;
            for sector in first_sector..first_sector + 8 {
                let mut data = self.disk.read(sector as usize);
                unsafe {
                    copy_nonoverlapping(data.as_mut_ptr(), (DISK_OFFSET + sector * 512) as *mut u8, 512)
                }
            }
        }
    }

    fn map_range(&mut self, low_addr: u64, high_addr: u64, load: bool) {
        debug_assert!(low_addr <= high_addr);
        debug_assert!(DISK_OFFSET <= low_addr);
        debug_assert!(high_addr <= DISK_OFFSET + PAGE_SIZE * self.get_num_pages() as u64);

        let low_page = low_addr / PAGE_SIZE;
        let high_page = (high_addr + PAGE_SIZE - 1) / PAGE_SIZE;

        for page in low_page..high_page {
            let page_addr = page * PAGE_SIZE;
            self.map_page(page_addr, load);
        }
    }

    pub fn declare_write(&mut self, low_addr: u64, high_addr: u64) {
        self.map_range(low_addr, high_addr, false);
    }

    pub fn declare_read(&mut self, low_addr: u64, high_addr: u64) {
        self.map_range(low_addr, high_addr, false);
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
        (self.get_is_taken().get_size_bytes() + PAGE_SIZE as usize - 1) / PAGE_SIZE as usize
    }

    pub fn erase(&mut self) {
        self.get_is_taken_mut().clear();
        for i in 0..=self.get_bitset_num_pages() {
            self.get_is_taken_mut().set(i, true);
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
            data.push(unsafe { *ptr.add(i) });
        }

        data
    }

    pub fn set_head(&mut self, data: &Vec<u8>) {
        self.declare_write(DISK_OFFSET, DISK_OFFSET + 4 + data.size() as u64);

        unsafe {
            *(DISK_OFFSET as *mut i32) = data.size() as i32;
        }

        let mut ptr = (DISK_OFFSET + 4) as *mut u8;
        for i in data {
            unsafe {
                *ptr = *i;
                ptr = ptr.add(1);
            }
        }
    }

    pub fn alloc_page(&mut self) -> i32 {
        let res = self.get_is_taken_mut().get_zero_element();
        if let Some(res) = res {
            self.get_is_taken_mut().set(res, true);
            res as i32
        } else {
            panic!("Out of disk space");
        }
    }

    pub fn free_page(&mut self, page: i32) {
        debug_assert!(self.get_is_taken().get(page as usize));
        self.get_is_taken_mut().set(page as usize, false);
    }
}

static mut MOUNTED_DISK: Option<Box<MemoryDisk>> = None;

pub fn unmount_disk() {
    if let Some(mounted_disk) = unsafe { MOUNTED_DISK.as_mut() } {
        unsafe {
            mounted_disk.is_taken.as_mut().unwrap().store_to(id_to_addr(1) as *mut u64);
        }

        for page in mounted_disk.mapped_pages.clone() {
            mounted_disk.unmap_page(page);
        }

        unsafe {
            MOUNTED_DISK = None;
        }
    }
}

pub fn mount_disk(disk: &'static mut Disk) {
    unmount_disk();

    let mounted_disk = Box::new(MemoryDisk::new(disk));
    unsafe {
        MOUNTED_DISK = Some(mounted_disk);
        MOUNTED_DISK.as_mut().unwrap().init();
    }
}

pub fn get_mounted_disk() -> &'static mut MemoryDisk {
    unsafe {
        if let Some(mounted_disk) = MOUNTED_DISK.as_mut() {
            mounted_disk
        } else {
            panic!("No disk is mounted.");
        }
    }
}

pub fn declare_disk_read(addr: u64) {
    debug_assert!(addr >= DISK_OFFSET);
    debug_assert!(addr < DISK_OFFSET + PAGE_SIZE * get_mounted_disk().get_num_pages() as u64);


}

pub fn declare_disk_write(addr: u64) {
    debug_assert!(addr >= DISK_OFFSET);
    debug_assert!(addr < DISK_OFFSET + PAGE_SIZE * get_mounted_disk().get_num_pages() as u64);


}

pub struct DiskBox<T: Serial> {
    size: i32,
    pub pages: Vec<i32>,
    obj: Option<T>,
}

impl<T: Serial> Serial for DiskBox<T> {
    fn serialize(&mut self, vec: &mut Vec<u8>) {
        if self.obj.is_some() {
            self.save();
        }
        self.size.serialize(vec);
        self.pages.serialize(vec);
        self.pages = Vec::new();
        self.obj = None;
    }

    fn deserialize(vec: &Vec<u8>, idx: &mut usize) -> Self {
        let size = i32::deserialize(vec, idx);
        let pages = Vec::<i32>::deserialize(vec, idx);
        debug_assert_eq!((size + PAGE_SIZE as i32 - 1) / PAGE_SIZE as i32, pages.size() as i32);

        Self { size, pages, obj: None }
    }
}

impl<T: Serial> DiskBox<T> {
    pub fn new(obj: T) -> Self {
        Self {
            pages: Vec::new(),
            size: 0,
            obj: Some(obj),
        }
    }

    fn save(&mut self) {
        for page in &self.pages {
            get_mounted_disk().free_page(*page);
        }
        self.pages = Vec::new();
        let data = serialize(self.obj.as_mut().unwrap());
        self.size = data.size() as i32;

        let mut idx = 0;
        while idx != data.size() {
            let curr_size = usize::min(PAGE_SIZE as usize, data.size() - idx);
            let page = get_mounted_disk().alloc_page();
            self.pages.push(page);
            unsafe {
                copy_nonoverlapping(data.get_unchecked(idx), id_to_addr(page), curr_size);
            }
            idx += curr_size;
        }
    }

    // translate idx-th byte to its ram location
    fn translate(&self, idx: usize) -> *mut u8 {
        debug_assert!(idx < self.size as usize);

        let page_id = self.pages[idx / (PAGE_SIZE as usize)];
        let page_addr = id_to_addr(page_id);
        unsafe { page_addr.add(idx % (PAGE_SIZE as usize)) }
    }

    pub fn get(&mut self) -> &mut T {
        if self.obj.is_some() {
            self.obj.as_mut().unwrap()
        } else {
            let mut data = Vec::new();
            for i in 0..self.size {
                data.push(unsafe { *self.translate(i as usize) });
            }

            let obj = deserialize(&data);
            self.obj = Some(obj);
            self.obj.as_mut().unwrap()
        }
    }

    // same as *get() = obj, but does not load it from disk
    pub fn set(&mut self, obj: T) {
        self.obj = Some(obj);
    }

    pub fn delete(mut self) {
        for page in &self.pages {
            get_mounted_disk().free_page(*page);
        }
        self.pages = Vec::new();
        self.obj = None;
    }
}

impl<T: Serial> Drop for DiskBox<T> {
    fn drop(&mut self) {
        if self.obj.is_some() {
            self.save();
        }
    }
}