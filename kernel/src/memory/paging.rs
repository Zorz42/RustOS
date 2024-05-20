use crate::memory::bitset::BitSetRaw;
use crate::memory::{PAGE_SIZE, VIRTUAL_OFFSET};
use crate::{print, println};
use std::memset_int64;

pub type PhysAddr = u64;
pub type VirtAddr = *mut u8;

// bit is 1 if the page is used, 0 if it's free
pub static mut SEGMENTS_BITSET: BitSetRaw = BitSetRaw::new_empty();

pub fn get_num_free_pages() -> usize {
    unsafe { SEGMENTS_BITSET.get_count0() }
}

pub fn get_num_pages() -> usize {
    unsafe { SEGMENTS_BITSET.get_size() }
}

type PageTableEntry = u64;

const PAGE_TABLE_SIZE: usize = 512;

#[repr(C)]
pub struct PageTable {
    entries: [PageTableEntry; PAGE_TABLE_SIZE],
}

impl PageTable {
    fn get_sub_page_table(&mut self, index: usize) -> Option<PhysAddr> {
        debug_assert!(index < PAGE_TABLE_SIZE);
        let entry = self.entries[index];
        if entry & 1 == 0 {
            None
        } else {
            Some(entry & 0x000FFFFF_FFFFF000)
        }
    }
}

fn create_page_table_entry(addr: PhysAddr, present: bool, writable: bool, user: bool) -> PageTableEntry {
    debug_assert_eq!(addr & 0xFFF00000_00000FFF, 0);
    let mut entry = addr & 0x000FFFFF_FFFFF000;
    if present {
        entry |= 1 << 0; // present
    }
    if writable {
        entry |= 1 << 1;
    }
    if user {
        entry |= 1 << 2;
    }
    entry
}

pub static mut CURRENT_PAGE_TABLE: *mut PageTable = 0 as *mut PageTable;

pub fn find_free_page() -> PhysAddr {
    unsafe {
        let index = SEGMENTS_BITSET.get_zero_element();
        if let Some(index) = index {
            SEGMENTS_BITSET.set(index, true);
            index as u64 * PAGE_SIZE
        } else {
            panic!("Out of memory");
        }
    }
}

pub unsafe fn clear_page_memory(addr: VirtAddr) {
    let addr = addr as u64 / PAGE_SIZE * PAGE_SIZE;
    memset_int64(addr as *mut u8, 0, PAGE_SIZE as usize);
}

pub unsafe fn free_page(addr: PhysAddr) {
    let index = (addr / PAGE_SIZE) as usize;
    assert!(SEGMENTS_BITSET.get(index), "Double free of page");
    SEGMENTS_BITSET.set(index, false);
}

fn get_address_page_table(virtual_addr: VirtAddr) -> *mut PageTable {
    let mut curr_table = unsafe { CURRENT_PAGE_TABLE };
    for i in 0..3 {
        let index = (virtual_addr as u64 >> (39 - 9 * i)) & 0b111111111;
        unsafe {
            if let Some(sub_table) = (*curr_table).get_sub_page_table(index as usize) {
                curr_table = (sub_table + VIRTUAL_OFFSET) as *mut PageTable;
            } else {
                let new_table = find_free_page();
                clear_page_memory((new_table + VIRTUAL_OFFSET) as VirtAddr);
                (*curr_table).entries[index as usize] = create_page_table_entry(new_table, true, true, false);
                curr_table = (new_table + VIRTUAL_OFFSET) as *mut PageTable;
            }
        }
    }
    
    curr_table
}

pub fn map_page(virtual_addr: VirtAddr, physical_addr: PhysAddr, writable: bool, user: bool) {
    let curr_table = get_address_page_table(virtual_addr);

    unsafe {
        let index = (virtual_addr as u64 >> 12) & 0b111111111;
        if (*curr_table).get_sub_page_table(index as usize).is_none() {
            (*curr_table).entries[index as usize] = create_page_table_entry(physical_addr, true, writable, user);
        }
    }
}

pub fn map_page_auto(virtual_addr: VirtAddr, writable: bool, user: bool) {
    map_page(virtual_addr, find_free_page(), writable, user);
}

pub fn unmap_page(virtual_addr: VirtAddr) {
    let curr_table = get_address_page_table(virtual_addr);

    unsafe {
        let index = (virtual_addr as u64 >> 12) & 0b111111111;
        if (*curr_table).get_sub_page_table(index as usize).is_none() {
            panic!("Cannot unmap non-present page");
        }
        (*curr_table).entries[index as usize] = create_page_table_entry(0, false, false, false);
    }
}

pub fn check_page_table_integrity() {
    #[cfg(debug_assertions)]
    {
        print!("Checking page table integrity ... ");

        // first 4 entries will be used by the kernel and will be identical for all page tables
        for i in 4..PAGE_TABLE_SIZE {
            let entry = unsafe { (*CURRENT_PAGE_TABLE).get_sub_page_table(i) };
            assert!(entry.is_none());
        }

        println!("OK");
    }
}
