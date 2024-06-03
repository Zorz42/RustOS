pub type PhysAddr = u64;
pub type VirtAddr = *mut u8;

use core::intrinsics::write_bytes;
use crate::boot::{NUM_CORES, STACK_SIZE};
use crate::memory::bitset::{bitset_size_bytes, BitSetRaw};
use crate::memory::{get_kernel_top_address, KERNEL_OFFSET, NUM_PAGES, PAGE_SIZE};
use crate::println;
use crate::riscv::{fence, set_satp};

pub static mut SEGMENTS_BITSET: BitSetRaw = BitSetRaw::new_empty();

pub fn get_num_free_pages() -> u64 {
    unsafe {
        SEGMENTS_BITSET.get_count0() as u64
    }
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

pub fn init_paging() {
    // for now just add 20 pages because apparently kernel writes after the end for some reason
    let mut kernel_end = (get_kernel_top_address() + 20 * PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;
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

    unsafe {
        CURRENT_PAGE_TABLE = create_page_table();
        switch_to_page_table(CURRENT_PAGE_TABLE);
    }
}

pub type PageTableEntry = u64;
pub type PageTable = *mut PageTableEntry;

const PAGE_TABLE_SIZE: usize = 512;

static mut CURRENT_PAGE_TABLE: PageTable = 0 as PageTable;

fn create_page_table() -> PageTable {
    let page_table = alloc_page() as PageTable;
    unsafe {
        write_bytes(page_table as *mut u8, 0, PAGE_SIZE as usize);
        *page_table = 0b1111;
    }

    page_table
}

fn switch_to_page_table(page_table: PageTable) {
    debug_assert_eq!(page_table as u64 % PAGE_SIZE, 0);
    fence();
    set_satp((page_table as u64 / PAGE_SIZE) | (9u64 << 60));
    fence();
}

/*pub fn refresh_paging() {
    unsafe {
        let cr3: u64;
        asm!("mov {}, cr3", out(reg) cr3);
        asm!("mov cr3, {}", in(reg) cr3);
    }
}

fn get_sub_page_table_entry(table: PageTable, index: usize) -> &'static mut PageTableEntry {
    debug_assert!(index < PAGE_TABLE_SIZE);
    unsafe {
        &mut *table.add(index)
    }
}

fn get_sub_page_table(table: PageTable, index: usize) -> Option<PageTable> {
    let entry = *get_sub_page_table_entry(table, index);
    if entry & 1 == 0 {
        None
    } else {
        Some((entry & 0x000FFFFF_FFFFF000) as PageTable)
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

fn get_address_page_table(virtual_addr: VirtAddr) -> PageTable {
    let mut curr_table = unsafe { CURRENT_PAGE_TABLE };
    for i in 0..3 {
        let index = (virtual_addr as u64 >> (39 - 9 * i)) & 0b111111111;
        unsafe {
            if let Some(sub_table) = get_sub_page_table(curr_table, index as usize) {
                curr_table = (sub_table as u64 + VIRTUAL_OFFSET) as PageTable;
            } else {
                let new_table = find_free_page();
                clear_page_memory((new_table + VIRTUAL_OFFSET) as VirtAddr);
                *get_sub_page_table_entry(curr_table, index as usize) = create_page_table_entry(new_table, true, true, false);
                curr_table = (new_table + VIRTUAL_OFFSET) as PageTable;
            }
        }
    }

    curr_table
}

pub fn map_page(virtual_addr: VirtAddr, physical_addr: PhysAddr, writable: bool, user: bool) {
    let curr_table = get_address_page_table(virtual_addr);

    unsafe {
        let index = (virtual_addr as u64 >> 12) & 0b111111111;
        if get_sub_page_table(curr_table, index as usize).is_none() {
            *get_sub_page_table_entry(curr_table, index as usize) = create_page_table_entry(physical_addr, true, writable, user);
        }
        debug_assert!(get_sub_page_table(curr_table, index as usize).is_some());
    }
    refresh_paging();
}

pub fn map_page_auto(virtual_addr: VirtAddr, writable: bool, user: bool) {
    map_page(virtual_addr, find_free_page(), writable, user);
}

pub fn unmap_page(virtual_addr: VirtAddr) {
    let curr_table = get_address_page_table(virtual_addr);

    unsafe {
        let index = (virtual_addr as u64 >> 12) & 0b111111111;
        if get_sub_page_table(curr_table, index as usize).is_none() {
            panic!("Cannot unmap non-present page");
        }
        *get_sub_page_table_entry(curr_table, index as usize) = create_page_table_entry(0, false, false, false);
        debug_assert!(get_sub_page_table(curr_table, index as usize).is_none());
    }
    refresh_paging();
}

pub fn check_page_table_integrity() {
    #[cfg(debug_assertions)]
    {
        print!("Checking page table integrity ... ");

        // first 4 entries will be used by the kernel and will be identical for all page tables
        for i in 4..PAGE_TABLE_SIZE {
            let entry = unsafe { get_sub_page_table(CURRENT_PAGE_TABLE, i) };
            assert!(entry.is_none());
        }

        println!("OK");
    }
}*/