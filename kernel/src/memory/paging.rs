use crate::memory::{memset_int64, PAGE_SIZE, VIRTUAL_OFFSET};
use crate::memory::bitset::BitSetRaw;

// bit is 1 if the page is used, 0 if it's free
pub static mut SEGMENTS_BITSET: BitSetRaw = BitSetRaw::new_empty();

pub fn get_num_free_pages() -> usize {
    unsafe { SEGMENTS_BITSET.get_count0() }
}

pub fn get_num_pages() -> usize {
    unsafe { SEGMENTS_BITSET.get_size() }
}

type PageTableEntry = u64;

pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    fn get_sub_page_table(&mut self, index: usize) -> Option<*mut PageTable> {
        let entry = self.entries[index];
        if entry & 1 == 0 {
            None
        } else {
            let addr = entry & 0x000FFFFF_FFFFF000;
            Some(addr as *mut PageTable)
        }
    }
}

fn create_page_table_entry(addr: u64, writable: bool, user: bool) -> PageTableEntry {
    let mut entry = addr & 0x000FFFFF_FFFFF000;
    entry |= 1 << 0; // present
    if writable {
        entry |= 1 << 1;
    }
    if user {
        entry |= 1 << 2;
    }
    entry
}

pub static mut CURRENT_PAGE_TABLE: *mut PageTable = 0 as *mut PageTable;

pub fn find_free_page() -> *mut u8 {
    unsafe {
        let index = SEGMENTS_BITSET.get_zero_element();
        if let Some(index) = index {
            SEGMENTS_BITSET.set(index, true);
            (index as u64 * PAGE_SIZE) as *mut u8
        } else {
            panic!("Out of memory");
        }
    }
}

pub unsafe fn clear_page_memory(addr: *mut u8) {
    let addr = addr as u64 / PAGE_SIZE * PAGE_SIZE;
    memset_int64(addr as *mut u8, 0, PAGE_SIZE as usize);
}

pub unsafe fn free_page(addr: *mut u8) {
    let index = (addr as u64 / PAGE_SIZE) as usize;
    assert!(SEGMENTS_BITSET.get(index), "Double free of page");
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
                (*curr_table).entries[index as usize] =
                    create_page_table_entry(new_table as u64, false, false);
                curr_table = (new_table as u64 + VIRTUAL_OFFSET) as *mut PageTable;
            }
        }
    }

    unsafe {
        let index = (virtual_addr >> 12) & 0b111111111;
        (*curr_table).entries[index as usize] =
            create_page_table_entry(physical_addr, writable, user);
    }
}
