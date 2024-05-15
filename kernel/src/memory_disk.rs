use std::{deserialize, memcpy_non_aligned, Serial, serialize, Vec};

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

fn id_to_addr(page: i32) -> *mut u8 {
    (DISK_OFFSET + page as u64 * PAGE_SIZE) as *mut u8
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
    
    pub fn get_size_left(&self) -> i32 {
        self.size_left
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

    pub fn get_master_page(&self) -> i32 {
        unsafe { *(DISK_OFFSET as *const i32) }
    }

    pub fn get_num_pages(&self) -> usize {
        self.disk.size() / 4
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
    }

    // bitset size in pages
    fn get_bitset_size(&self) -> usize {
        (self.bitset.get_size_bytes() + PAGE_SIZE as usize - 1) / PAGE_SIZE as usize
    }

    pub fn erase(&mut self) {
        self.bitset.clear();
        for i in 0..=self.get_bitset_size() {
            self.bitset.set(i, true);
        }
        unsafe {
            *(DISK_OFFSET as *mut i32) = self.create();
        }
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
        let addr = id_to_addr(page) as *mut i32;
        unsafe {
            *addr = 0;
        }
        page
    }

    pub fn destroy(&mut self, id: i32) {
        let mut iter = PageIterator::new(id_to_addr(id));
        loop {
            let curr_id = (iter.get_curr_addr() as u64 - DISK_OFFSET) / PAGE_SIZE;
            debug_assert!(self.bitset.get(curr_id as usize));
            self.bitset.set(curr_id as usize, false);
            
            if !iter.advance() {
                break;
            }
        }
    }

    pub fn save(&mut self, id: i32, data: &Vec<u8>) {
        self.destroy(id);
        let addr = id_to_addr(id);
        unsafe {
            *(addr as *mut i32) = data.size() as i32;
        }
        let mut iter = PageIterator::new(addr);
        let mut i = 0;
        loop {
            let curr_size = usize::min(data.size() - i, iter.get_curr_size() as usize);
            unsafe {
                memcpy_non_aligned(data.get_unchecked(i), iter.get_curr_addr(), curr_size);
            }

            let page = (iter.get_curr_addr() as u64 - DISK_OFFSET) / PAGE_SIZE;
            self.bitset.set(page as usize, true);
            
            i += curr_size;
            if i == data.size() {
                break;
            }
            
            set_next_page(page as i32, self.alloc_page());
            
            
            assert!(iter.advance());
        }
    }

    pub fn load(&mut self, id: i32) -> Vec<u8> {
        let mut res = Vec::new();
        
        let mut iter = PageIterator::new(id_to_addr(id));
        
        loop {
            let size = i32::min(iter.get_size_left(), iter.get_curr_size());
            for i in 0..size {
                unsafe {
                    res.push(*iter.get_curr_addr().add(i as usize));
                }
            }
            
            if !iter.advance() {
                break;
            }
        }
        
        res
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

    mounted_disk.map_page(addr);

    true
}

pub struct DiskBox<T: Serial> {
    page: i32,
    obj: Option<T>,
}

impl<T: Serial> Serial for DiskBox<T> {
    fn serialize(&self, vec: &mut Vec<u8>) {
        self.page.serialize(vec);
    }

    fn deserialize(vec: &Vec<u8>, idx: &mut usize) -> Self {
        Self::new_at_page(i32::deserialize(vec, idx))
    }
}

impl<T: Serial> DiskBox<T> {
    pub fn new(obj: T) -> Self {
        Self {
            page: get_mounted_disk().create(),
            obj: Some(obj),
        }
    }
    
    pub fn new_at_page(page: i32) -> Self {
        Self {
            page,
            obj: None,
        }
    }
    
    pub fn get(&mut self) -> &mut T {
        if self.obj.is_some() {
            self.obj.as_mut().unwrap()
        } else {
            let obj = deserialize(&get_mounted_disk().load(self.page));
            self.obj = Some(obj);
            self.obj.as_mut().unwrap()
        }
    }

    // same as *get() = obj, but does not load it from disk
    pub fn set(&mut self, obj: T) {
        self.obj = Some(obj);
    }
    
    pub fn delete(mut self) {
        get_mounted_disk().destroy(self.page);
        self.obj = None;
    }
}

impl<T: Serial> Drop for DiskBox<T> {
    fn drop(&mut self) {
        if let Some(obj) = &self.obj {
            get_mounted_disk().save(self.page, &serialize(obj));
        }
    }
}
 