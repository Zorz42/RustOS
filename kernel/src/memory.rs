use bootloader_api::info::MemoryRegions;
use crate::println;

const PAGE_SIZE: u64 = 4096;

pub const VIRTUAL_OFFSET: u64 = 0x100000000;
pub const KERNEL_STACK_SIZE: u64 = 100 * 1024; // 100 KiB

struct BitSetRaw {
    data: *mut u64,
    size: usize,
    count1: usize,
}

impl BitSetRaw {
    const fn new(size: usize, addr: *mut u64) -> BitSetRaw {
        BitSetRaw {
            data: addr,
            size: (size + 63) / 64 * 64,
            count1: 0,
        }
    }

    fn set(&mut self, index: usize, val: bool) {
        debug_assert!(index < self.size);
        
        let byte_index = index / 64;
        let bit_index = index % 64;
        self.count1 += val as usize;
        self.count1 -= self.get(index) as usize;
        unsafe {
            if val {
                *self.data.offset(byte_index as isize) |= 1 << bit_index;
            } else {
                *self.data.offset(byte_index as isize) &= !(1 << bit_index);
            }
        }
    }

    fn get(&self, index: usize) -> bool {
        debug_assert!(index < self.size);
        
        let byte_index = index / 64;
        let bit_index = index % 64;
        unsafe {
            (*self.data.offset(byte_index as isize) & (1 << bit_index)) != 0
        }
    }
    
    fn get_size_bytes(&self) -> usize {
        self.size / 8
    }
    
    fn get_first_unset(&self) -> Option<usize> {
        for i in 0..self.size / 64 {
            let mut val = unsafe { *self.data.offset(i as isize) };
            val = !val;
            if val != 0 {
                return Some(i * 64 + val.trailing_zeros() as usize);
            }
        }
        None
    }
    
    fn clear(&mut self) {
        for i in 0..self.size / 64 {
            unsafe {
                *self.data.offset(i as isize) = 0;
            }
        }
        self.count1 = 0;
    }
}

// bit is 1, if page is free
static mut SEGMENTS_BITSET: BitSetRaw = BitSetRaw::new(0, 0 as *mut u64);

pub fn get_num_free_pages() -> usize {
    unsafe {
        SEGMENTS_BITSET.count1
    }
}

pub fn get_num_pages() -> usize {
    unsafe {
        SEGMENTS_BITSET.size
    }
}

pub fn init_memory(memory_regions: &MemoryRegions) {
    let mut highest_address = 0;
    for region in memory_regions.iter() {
        highest_address = highest_address.max(region.end);
    }
    let num_all_pages = highest_address / PAGE_SIZE;
    // how many pages will the bitset alone take
    let pages_for_bitset = (num_all_pages + 8 * PAGE_SIZE - 1) / (8 * PAGE_SIZE);
    
    let bitset_addr = {
        // find some consecutive free pages in memory_regions
        let mut bitset_addr = None;
        for region in memory_regions.iter() {
            let start_page = (region.start + PAGE_SIZE - 1) / PAGE_SIZE;
            let end_page = region.end / PAGE_SIZE;
            if end_page - start_page >= pages_for_bitset {
                bitset_addr = Some(start_page * PAGE_SIZE);
                break;
            }
        }
        if let Some(addr) = bitset_addr {
            addr
        } else {
            panic!("Could not find enough free pages for the bitset");
        }
    };
    
    unsafe {
        SEGMENTS_BITSET = BitSetRaw::new(num_all_pages as usize, (VIRTUAL_OFFSET + bitset_addr) as *mut u64);
        SEGMENTS_BITSET.clear();
    }
    
    // mark every free page as free
    for region in memory_regions.iter() {
        let start_page = (region.start + PAGE_SIZE - 1) / PAGE_SIZE;
        let end_page = region.end / PAGE_SIZE;
        for page in start_page..end_page {
            unsafe {
                SEGMENTS_BITSET.set(page as usize, true);
            }
        }
    }
    
    // mark the pages for the bitset as used
    {
        let bitset_first_page = bitset_addr / PAGE_SIZE;
        let bitset_last_page = bitset_first_page + pages_for_bitset;
        for page in bitset_first_page..bitset_last_page {
            unsafe {
                SEGMENTS_BITSET.set(page as usize, false);
            }
        }
    }
    
    println!("Num pages {}, num free pages {}", get_num_pages(), get_num_free_pages());
}