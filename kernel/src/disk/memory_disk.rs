use core::ops::Deref;
use core::ptr::write_volatile;
use kernel_std::{Vec, Mutable, Box, println};

use crate::disk::disk::{Disk, SECTOR_SIZE};
use crate::memory::BitSet;

pub struct MemoryDisk {
    disk: Disk,
    is_taken: BitSet, // which page is taken
    cache: Vec<Option<Box<[u8; SECTOR_SIZE]>>>,
}

impl MemoryDisk {
    pub fn new(disk: &Disk) -> Self {
        let mut res = Self {
            disk: disk.clone(),
            is_taken: BitSet::new(0),
            cache: Vec::new(),
        };

        for _ in 0..res.get_num_sectors() {
            res.cache.push(None);
        }

        res.is_taken = BitSet::new(res.disk.size());
        let mut data = Vec::new();
        for i in 0..res.get_bitset_num_sectors() {
            let sector = res.read_sector(i + 1);
            for j in sector {
                data.push(j);
            }
        }

        unsafe {
            res.is_taken.load_from(data.as_mut_ptr() as *mut u64);
        }

        res
    }

    pub const fn get_num_sectors(&self) -> usize {
        self.disk.size()
    }

    pub fn get_num_free_sectors(&self) -> usize {
        self.is_taken.get_count0()
    }

    pub const fn get_size(&self) -> usize {
        self.get_num_sectors() * SECTOR_SIZE
    }

    // bitset size in pages
    fn get_bitset_num_sectors(&self) -> usize {
        self.is_taken.get_size_bytes().div_ceil(SECTOR_SIZE)
    }

    pub fn erase(&mut self) {
        self.is_taken.clear();
        for i in 0..=self.get_bitset_num_sectors() {
            self.is_taken.set(i, true);
        }

        self.set_head(&Vec::new());
    }

    pub fn read_sector(&mut self, sector: usize) -> [u8; SECTOR_SIZE] {
        if self.cache[sector].is_none() {
            self.cache[sector] = Some(Box::new(self.disk.read(sector)));
        }
        self.cache[sector].as_ref().unwrap().deref().clone()
    }

    pub fn write_sector(&mut self, sector: usize, data: &[u8; SECTOR_SIZE]) {
        self.cache[sector] = Some(Box::new(*data));
        self.disk.write(sector, data);
    }

    pub fn get_head(&mut self) -> Vec<u8> {
        let first_sector = self.read_sector(0);

        let size = unsafe { *(&first_sector[0] as *const u8 as *const i32) } as usize;
        let mut data = Vec::new();

        for i in 0..size {
            data.push(first_sector[i + 4]);
        }

        data
    }

    pub fn set_head(&mut self, data: &Vec<u8>) {
        let mut first_sector = self.read_sector(0);

        unsafe {
            write_volatile(&mut first_sector[0] as *mut u8 as *mut i32, data.size() as i32);
        }

        for i in 0..data.size() {
            first_sector[i + 4] = *data.get(i).unwrap();
        }

        self.write_sector(0, &first_sector);
    }

    pub fn alloc_sector(&mut self) -> usize {
        let res = self.is_taken.get_zero_element();
        if let Some(res) = res {
            self.is_taken.set(res, true);
            res
        } else {
            panic!("Out of disk space");
        }
    }

    pub fn free_sector(&mut self, sector: usize) {
        debug_assert!(self.is_taken.get(sector));
        self.is_taken.set(sector, false);
    }

    pub fn flush_cache(&mut self, sector: usize) {
        if let Some(sector_data) = self.cache[sector].as_ref() {
            let mut data = [0; SECTOR_SIZE];
            for i in 0..SECTOR_SIZE {
                data[i] = sector_data[i];
            }
            self.disk.write(sector, &data);
        }
        self.cache[sector] = None;
    }
}

static MOUNTED_DISK: Mutable<Option<MemoryDisk>> = Mutable::new(None);

pub fn unmount_disk() {
    let t = MOUNTED_DISK.borrow();
    if let Some(mounted_disk) = MOUNTED_DISK.get_mut(&t) {
        let num_bitset_sectors = mounted_disk.get_bitset_num_sectors();
        let mut data = unsafe { Vec::new_with_size_uninit(num_bitset_sectors * SECTOR_SIZE) };
        unsafe {
            mounted_disk.is_taken.store_to(data.as_mut_ptr() as *mut u64);
        }

        for i in 0..num_bitset_sectors {
            let mut sector = [0; SECTOR_SIZE];
            for j in 0..SECTOR_SIZE {
                if i * SECTOR_SIZE + j < data.size() {
                    sector[j] = data[i * SECTOR_SIZE + j];
                }
            }
            mounted_disk.write_sector(i + 1, &sector);
        }

        for i in 0..mounted_disk.get_num_sectors() {
            mounted_disk.flush_cache(i);
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
    MOUNTED_DISK.release(t);
}

pub fn get_mounted_disk() -> &'static Mutable<Option<MemoryDisk>> {
    &MOUNTED_DISK
}