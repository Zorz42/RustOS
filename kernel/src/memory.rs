use core::arch::asm;
use bootloader_api::info::MemoryRegions;
use crate::{print, println};

const PAGE_SIZE: u64 = 4096;

pub const VIRTUAL_OFFSET: u64 = 0x100000000;
pub const KERNEL_STACK_SIZE: u64 = 100 * 1024; // 100 KiB
pub const KERNEL_STACK_ADDR: u64 = 0x200000000 - KERNEL_STACK_SIZE;

struct BitSetRaw {
    data: *mut u64,
    size: usize,
    count0: usize,
}

impl BitSetRaw {
    const fn new(size: usize, addr: *mut u64) -> BitSetRaw {
        BitSetRaw {
            data: addr,
            size: (size + 63) / 64 * 64,
            count0: 0,
        }
    }

    fn set(&mut self, index: usize, val: bool) {
        debug_assert!(index < self.size);
        
        let byte_index = index / 64;
        let bit_index = index % 64;
        self.count0 += val as usize ^ 1;
        self.count0 -= self.get(index) as usize ^ 1;
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
    
    fn get_first_zero(&self) -> Option<usize> {
        for i in 0..self.size / 64 {
            let mut val = unsafe { *self.data.offset(i as isize) };
            if val != 0xFFFFFFFF_FFFFFFFF {
                for j in 0..64 {
                    if val & 1 == 0 {
                        return Some(i * 64 + j);
                    }
                    val >>= 1;
                }
                unreachable!();
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
        self.count0 = self.size;
    }
}

// bit is 1 if the page is used, 0 if it's free
static mut SEGMENTS_BITSET: BitSetRaw = BitSetRaw::new(0, 0 as *mut u64);

pub fn get_num_free_pages() -> usize {
    unsafe {
        SEGMENTS_BITSET.count0
    }
}

pub fn get_num_pages() -> usize {
    unsafe {
        SEGMENTS_BITSET.size
    }
}

type PageTableEntry = u64;

struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    fn get_sub_page_table(&mut self, index: usize) -> Option<*mut PageTable> {
        let entry = self.entries[index];
        if entry & 1 == 0 {
            None
        } else {
            let addr = entry & 0x000FFFFF_FFFFF000;
            Some(unsafe { addr as *mut PageTable })
        }
    }
}

fn create_page_table_entry(addr: u64, writable: bool, user: bool) -> PageTableEntry {
    let mut entry = (addr << 12) & 0x000FFFFF_FFFFF000;
    entry |= 1; // present
    if writable {
        entry |= 1 << 1; // writable
    }
    if user {
        entry |= 1 << 2; // user
    }
    entry
}

static mut CURRENT_PAGE_TABLE: *mut PageTable = 0 as *mut PageTable;

pub fn find_free_page() -> *mut u8 {
    unsafe {
        let index = SEGMENTS_BITSET.get_first_zero();
        if let Some(index) = index {
            SEGMENTS_BITSET.set(index, true);
            (index as u64 * PAGE_SIZE) as *mut u8
        } else {
            panic!("Out of memory");
        }
    }
}

pub unsafe fn clear_page_memory(addr: *mut u8) {
    let addr = addr as *mut u64;
    for i in 0..PAGE_SIZE / 8 {
        *addr.offset(i as isize) = 0;
    }
}

pub unsafe fn free_page(addr: *mut u8) {
    let index = (addr as u64 / PAGE_SIZE) as usize;
    assert!(!SEGMENTS_BITSET.get(index), "Double free of page");
    SEGMENTS_BITSET.set(index, false);
}

pub fn map_page(virtual_addr: u64, physical_addr: u64, writable: bool, user: bool) {
    let mut curr_table = unsafe { CURRENT_PAGE_TABLE };
    for i in 0..3 {
        let index = (virtual_addr >> (39 - 9 * i)) & 0b111111111;
        unsafe {
            if let Some(sub_table) = (*curr_table).get_sub_page_table(index as usize) {
                curr_table = (sub_table as u64 + VIRTUAL_OFFSET) as *mut PageTable;
            } else {
                let new_table = find_free_page() as *mut PageTable;
                clear_page_memory((new_table as u64 + VIRTUAL_OFFSET) as *mut u8);
                (*curr_table).entries[index as usize] = create_page_table_entry(new_table as u64, false, false);
                curr_table = (new_table as u64 + VIRTUAL_OFFSET) as *mut PageTable;
            }
        }
    }
    
    unsafe {
        let index = (virtual_addr >> 12) & 0x1FF;
        (*curr_table).entries[index as usize] = create_page_table_entry(physical_addr, writable, user);
    }
}

unsafe fn switch_to_page_table(page_table: u64) {
    asm!("mov cr3, {}", in(reg) page_table);
    loop {}
}

pub fn init_memory(memory_regions: &MemoryRegions, framebuffer: u64, framebuffer_size: u64, kernel_phys: u64, kernel_virt: u64, kernel_len: u64) {
    unsafe {
        // use already existing page table
        let cr3: u64;
        asm!("mov {}, cr3", out(reg) cr3);
        CURRENT_PAGE_TABLE = (cr3 + VIRTUAL_OFFSET) as *mut PageTable;
    }
    
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
            if region.kind != bootloader_api::info::MemoryRegionKind::Usable || region.start == 0 {
                continue;
            }
            
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

    // mark the pages for the bitset as used
    let bitset_first_page = bitset_addr / PAGE_SIZE;
    let bitset_last_page = bitset_first_page + pages_for_bitset;
    for page in bitset_first_page..bitset_last_page {
        unsafe {
            SEGMENTS_BITSET.set(page as usize, true);
        }
    }
    
    // mark every page, that is already used, as used
    for region in memory_regions.iter() {
        if region.kind == bootloader_api::info::MemoryRegionKind::Usable && region.start != 0 {
            continue;
        }
        
        let start_page = region.start / PAGE_SIZE;
        let end_page = (region.end + PAGE_SIZE - 1) / PAGE_SIZE;
        for page in start_page..end_page {
            unsafe {
                SEGMENTS_BITSET.set(page as usize, true);
            }
        }
    }
    
    // map the bitset
    for page in bitset_first_page..bitset_last_page {
        unsafe {
            map_page(page * PAGE_SIZE + VIRTUAL_OFFSET, page * PAGE_SIZE, true, false);
        }
    }
}