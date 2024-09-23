pub type PhysAddr = u64;
pub type VirtAddr = *mut u8;

use crate::boot::{NUM_CORES, STACK_SIZE};
use crate::memory::bitset::{bitset_size_bytes, BitSetRaw};
use crate::memory::{get_kernel_top_address, HEAP_BASE_ADDR, HEAP_TREE_ADDR, KERNEL_OFFSET, NUM_PAGES, PAGE_SIZE};
use crate::riscv::{get_satp, set_satp};
use core::intrinsics::write_bytes;
use core::sync::atomic::{fence, Ordering};
use std::init_std_memory;

pub static mut SEGMENTS_BITSET: BitSetRaw = BitSetRaw::new_empty();

pub fn get_num_free_pages() -> u64 {
    unsafe { SEGMENTS_BITSET.get_count0() as u64 }
}

pub fn alloc_page() -> PhysAddr {
    unsafe {
        let index = SEGMENTS_BITSET.get_zero_element();
        if let Some(index) = index {
            SEGMENTS_BITSET.set(index, true);
            index as u64 * PAGE_SIZE + KERNEL_OFFSET
        } else {
            panic!("Out of memory");
        }
    }
}

pub fn free_page(addr: PhysAddr) {
    debug_assert!(addr >= KERNEL_OFFSET);
    let index = ((addr - KERNEL_OFFSET) / PAGE_SIZE) as usize;
    unsafe {
        assert!(SEGMENTS_BITSET.get(index));
        SEGMENTS_BITSET.set(index, false);
    }
}

fn page_allocator(page: VirtAddr, ignore_if_exists: bool) {
    map_page_auto(page, ignore_if_exists, true, false);
}

fn page_deallocator(page: VirtAddr) {
    unmap_page(page);
}

pub fn init_paging() {
    // for now just add 20 pages because apparently kernel writes after the end for some reason
    let kernel_end = (get_kernel_top_address() - 1) / PAGE_SIZE * PAGE_SIZE;
    let bitset_size_bytes = bitset_size_bytes(NUM_PAGES as usize);
    let bitset_size_pages = (bitset_size_bytes as u64 + PAGE_SIZE - 1) / PAGE_SIZE;
    let kernel_size_pages = (kernel_end - KERNEL_OFFSET) / PAGE_SIZE;

    unsafe {
        SEGMENTS_BITSET = BitSetRaw::new(NUM_PAGES as usize, kernel_end as *mut u64);
    }

    // mark kernel and bitset pages as taken
    for i in 0..bitset_size_pages + kernel_size_pages {
        unsafe {
            SEGMENTS_BITSET.set(i as usize, true);
        }
    }

    let stack_size = NUM_CORES * STACK_SIZE;
    let stack_size_pages = (stack_size as u64 + PAGE_SIZE - 1) / PAGE_SIZE;

    // mark stack pages as taken
    for i in 0..stack_size_pages {
        unsafe {
            SEGMENTS_BITSET.set((NUM_PAGES - 1 - i) as usize, true);
        }
    }

    let page_table = create_page_table();
    switch_to_page_table(page_table);

    init_std_memory(&page_allocator, &page_deallocator, HEAP_TREE_ADDR, HEAP_BASE_ADDR);
}

pub fn init_paging_hart() {
    unsafe {
        switch_to_page_table(CURRENT_PAGE_TABLE);
    }
}

pub type PageTableEntry = u64;
pub type PageTable = *mut PageTableEntry;

pub const PTE_PRESENT: u64 = 1;
pub const PTE_READ: u64 = 1 << 1;
pub const PTE_WRITE: u64 = 1 << 2;
pub const PTE_EXECUTE: u64 = 1 << 3;
pub const PTE_USER: u64 = 1 << 4;

const PAGE_TABLE_SIZE: usize = 512;

static mut CURRENT_PAGE_TABLE: PageTable = 0 as PageTable;

fn create_page_table() -> PageTable {
    let page_table = alloc_page() as PageTable;
    unsafe {
        write_bytes(page_table as *mut u8, 0, PAGE_SIZE as usize);
        for i in 0..3 {
            *page_table.add(i) = create_page_table_entry(((i as u64) << (12 + 2 * 9)) as PhysAddr) | PTE_READ | PTE_WRITE | PTE_EXECUTE;
        }
    }

    page_table
}

fn switch_to_page_table(page_table: PageTable) {
    debug_assert_eq!(page_table as u64 % PAGE_SIZE, 0);
    fence(Ordering::Release);
    unsafe {
        CURRENT_PAGE_TABLE = page_table;
    }
    set_satp((page_table as u64 / PAGE_SIZE) | (8u64 << 60));
    fence(Ordering::Release);
}

pub fn refresh_paging() {
    fence(Ordering::Release);
    let satp = get_satp();
    set_satp(satp);
    fence(Ordering::Release);
}

fn get_sub_page_table_entry(table: PageTable, index: usize) -> &'static mut PageTableEntry {
    debug_assert!(index < PAGE_TABLE_SIZE);
    unsafe { &mut *table.add(index) }
}

const fn get_entry_addr(entry: PageTableEntry) -> Option<PageTable> {
    if (entry & PTE_PRESENT) == 0 {
        None
    } else {
        Some(((entry >> 10) << 12) as PageTable)
    }
}

const fn is_entry_table(entry: PageTableEntry) -> bool {
    (entry & (PTE_PRESENT | PTE_READ | PTE_WRITE | PTE_EXECUTE)) == PTE_PRESENT
}

fn create_page_table_entry(addr: PhysAddr) -> PageTableEntry {
    debug_assert_eq!(addr & 0xFFFFF800_00000FFF, 0);
    ((addr >> 12) << 10) | PTE_PRESENT
}

fn get_address_page_table_entry(virtual_addr: VirtAddr) -> Option<&'static mut PageTableEntry> {
    let mut curr_table = unsafe { CURRENT_PAGE_TABLE };
    for i in 0..2 {
        let index = (virtual_addr as u64 >> (30 - 9 * i)) & 0b111111111;
        unsafe {
            let entry = get_sub_page_table_entry(curr_table, index as usize);
            if let Some(table) = get_entry_addr(*entry) {
                if !is_entry_table(*entry) {
                    return None;
                }
                curr_table = table;
            } else {
                let new_table = alloc_page();
                write_bytes(new_table as *mut u8, 0, PAGE_SIZE as usize);
                *get_sub_page_table_entry(curr_table, index as usize) = create_page_table_entry(new_table);
                curr_table = new_table as PageTable;
            }
        }
    }

    let index = (virtual_addr as u64 >> 12) & 0b111111111;
    Some(get_sub_page_table_entry(curr_table, index as usize))
}

pub fn map_page(virtual_addr: VirtAddr, physical_addr: PhysAddr, ignore_if_exists: bool, writable: bool, user: bool) {
    let curr_entry = get_address_page_table_entry(virtual_addr).unwrap();
    if ignore_if_exists && (*curr_entry & PTE_PRESENT) != 0 {
        return;
    }
    debug_assert_eq!(*curr_entry & PTE_PRESENT, 0);
    *curr_entry = create_page_table_entry(physical_addr) | PTE_READ;
    if writable {
        *curr_entry |= PTE_WRITE;
    }
    if user {
        *curr_entry |= PTE_USER;
    }
    refresh_paging();
}

pub fn map_page_auto(virtual_addr: VirtAddr, ignore_if_exists: bool, writable: bool, user: bool) {
    map_page(virtual_addr, alloc_page(), ignore_if_exists, writable, user);
}

pub fn virt_to_phys(addr: VirtAddr) -> Option<PhysAddr> {
    let entry = get_address_page_table_entry(addr).unwrap();
    Some(get_entry_addr(*entry)? as PhysAddr)
}

pub fn unmap_page(virtual_addr: VirtAddr) {
    let curr_entry = get_address_page_table_entry(virtual_addr).unwrap();
    debug_assert!((*curr_entry & PTE_PRESENT) == PTE_PRESENT);
    *curr_entry = 0;
    refresh_paging();
}
