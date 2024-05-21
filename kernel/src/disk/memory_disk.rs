use std::{deserialize, memcpy_non_aligned, serialize, Serial, Vec, Box};

use crate::disk::disk::Disk;
use crate::memory::{map_page_auto, BitSetRaw, VirtAddr, DISK_OFFSET, PAGE_SIZE, unmap_page};
use crate::println;

pub struct MemoryDisk {
    disk: Disk,
    mapped_pages: Vec<i32>,
    bitset: Option<BitSetRaw>, // which page is taken
}

fn id_to_addr(page: i32) -> *mut u8 {
    (DISK_OFFSET + page as u64 * PAGE_SIZE) as *mut u8
}

impl MemoryDisk {
    pub fn new(disk: Disk) -> Self {
        Self {
            disk,
            mapped_pages: Vec::new(),
            bitset: None,
        }
    }

    pub fn init(&mut self) {
        self.bitset = Some(BitSetRaw::new_from(self.disk.size() / 8, id_to_addr(1) as *mut u64));
    }

    fn get_bitset(&self) -> &BitSetRaw {
        self.bitset.as_ref().unwrap()
    }

    fn get_bitset_mut(&mut self) -> &mut BitSetRaw {
        self.bitset.as_mut().unwrap()
    }

    pub fn get_num_pages(&self) -> usize {
        self.disk.size() / 4
    }
    
    pub fn get_num_free_pages(&self) -> usize {
        self.bitset.as_ref().unwrap().get_count0()
    }

    pub fn get_size(&self) -> usize {
        self.get_num_pages() * PAGE_SIZE as usize
    }

    fn map_page(&mut self, addr: u64) {
        let idx = (addr - DISK_OFFSET) / PAGE_SIZE;
        self.mapped_pages.push(idx as i32);

        let addr = addr / PAGE_SIZE * PAGE_SIZE;
        map_page_auto(addr as VirtAddr, true, false);
        let first_sector = (addr - DISK_OFFSET) / PAGE_SIZE * 8;
        for sector in first_sector..first_sector + 8 {
            let mut data = self.disk.read(sector as i32);
            unsafe {
                memcpy_non_aligned(data.as_mut_ptr(), (DISK_OFFSET + sector * 512) as *mut u8, 512);
            }
        }
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
        unmap_page(id_to_addr(page));
    }

    // bitset size in pages
    fn get_bitset_size(&self) -> usize {
        (self.get_bitset().get_size_bytes() + PAGE_SIZE as usize - 1) / PAGE_SIZE as usize
    }

    pub fn erase(&mut self) {
        self.get_bitset_mut().clear();
        for i in 0..=self.get_bitset_size() {
            self.get_bitset_mut().set(i, true);
        }
        self.set_head(&Vec::new());
    }

    pub fn get_head(&mut self) -> Vec<u8> {
        let size = unsafe { *(DISK_OFFSET as *mut i32) } as usize;
        let mut data = Vec::new();

        let ptr = (DISK_OFFSET + 4) as *mut u8;
        for i in 0..size {
            data.push(unsafe { *ptr.add(i) });
        }

        data
    }

    pub fn set_head(&mut self, data: &Vec<u8>) {
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
        let res = self.get_bitset().get_zero_element();
        if let Some(res) = res {
            self.get_bitset_mut().set(res, true);
            res as i32
        } else {
            panic!("Out of disk space");
        }
    }

    pub fn free_page(&mut self, page: i32) {
        debug_assert!(self.get_bitset().get(page as usize));
        self.get_bitset_mut().set(page as usize, false);
    }
}

static mut MOUNTED_DISK: Option<Box<MemoryDisk>> = None;

pub fn unmount_disk() {
    if let Some(mounted_disk) = unsafe { MOUNTED_DISK.as_mut() } {
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

pub fn disk_page_fault_handler(addr: u64) -> bool {
    if addr < DISK_OFFSET || addr >= DISK_OFFSET + get_mounted_disk().get_size() as u64 {
        return false;
    }

    let mounted_disk = unsafe {
        if let Some(mounted_disk) = MOUNTED_DISK.as_mut() {
            mounted_disk
        } else {
            return false;
        }
    };

    mounted_disk.map_page(addr);

    true
}

pub struct DiskBox<T: Serial> {
    size: i32,
    pages: Vec<i32>,
    obj: Option<T>,
}

impl<T: Serial> Serial for DiskBox<T> {
    fn serialize(&mut self, vec: &mut Vec<u8>) {
        if self.obj.is_some() {
            self.save();
        }
        self.size.serialize(vec);
        self.pages.serialize(vec);
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
                memcpy_non_aligned(data.get_unchecked(idx), id_to_addr(page), curr_size);
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
