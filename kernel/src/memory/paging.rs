pub type PhysAddr = u64;
pub type VirtAddr = *mut u8;

use crate::boot::{NUM_CORES, STACK_SIZE};
use kernel_std::{bitset_size_bytes, BitSetRaw};
use crate::memory::{get_kernel_top_address, HEAP_ADDR, HEAP_BASE_ADDR, HEAP_TREE_ADDR, ID_MAP_END, KERNEL_OFFSET, KERNEL_PT_ROOT_ENTRIES, NUM_PAGES, PAGE_SIZE};
use crate::riscv::{get_core_id, get_satp, set_satp};
use core::intrinsics::write_bytes;
use core::sync::atomic::{fence, Ordering};
use kernel_std::init_std_memory;
use kernel_std::Mutable;

pub static SEGMENTS_BITSET: Mutable<BitSetRaw> = Mutable::new(BitSetRaw::new_empty());

pub fn get_num_free_pages() -> u64 {
    let t = SEGMENTS_BITSET.borrow();
    let res = SEGMENTS_BITSET.get(&t).get_count0() as u64;
    SEGMENTS_BITSET.release(t);
    res
}

pub fn alloc_page() -> PhysAddr {
    let t = SEGMENTS_BITSET.borrow();
    let index = SEGMENTS_BITSET.get_mut(&t).get_zero_element();
    if let Some(index) = index {
        SEGMENTS_BITSET.get_mut(&t).set(index, true);

        SEGMENTS_BITSET.release(t);
        index as u64 * PAGE_SIZE + KERNEL_OFFSET
    } else {
        panic!("Out of memory");
    }
}

pub fn alloc_continuous_pages(num: u64) -> PhysAddr {
    let t = SEGMENTS_BITSET.borrow();
    for i in 0..=NUM_PAGES - num {
        let mut all_free = true;
        for j in 0..num {
            if SEGMENTS_BITSET.get(&t).get(i as usize + j as usize) {
                all_free = false;
                break;
            }
        }
        if all_free {
            for k in 0..num {
                SEGMENTS_BITSET.get_mut(&t).set(i as usize + k as usize, true);
            }
            SEGMENTS_BITSET.release(t);
            return i * PAGE_SIZE + KERNEL_OFFSET;
        }
    }
    SEGMENTS_BITSET.release(t);
    panic!("Out of memory");
}

pub fn free_page(addr: PhysAddr) {
    debug_assert!(addr >= KERNEL_OFFSET);
    let index = ((addr - KERNEL_OFFSET) / PAGE_SIZE) as usize;
    let t = SEGMENTS_BITSET.borrow();
    assert!(SEGMENTS_BITSET.get(&t).get(index));
    SEGMENTS_BITSET.get_mut(&t).set(index, false);
    SEGMENTS_BITSET.release(t);
}

fn page_allocator(page: VirtAddr, ignore_if_exists: bool) {
    map_page_auto(page, ignore_if_exists, true, false, false);
}

fn page_deallocator(page: VirtAddr) {
    let page_addr = virt_to_phys(page).unwrap();
    unmap_page(page);
    free_page(page_addr);
}

pub fn init_paging() {
    // for now just add 20 pages because apparently kernel writes after the end for some reason
    let kernel_end = (get_kernel_top_address() - 1) / PAGE_SIZE * PAGE_SIZE;
    let bitset_size_bytes = bitset_size_bytes(NUM_PAGES as usize);
    let bitset_size_pages = (bitset_size_bytes as u64).div_ceil(PAGE_SIZE);
    let kernel_size_pages = (kernel_end - KERNEL_OFFSET) / PAGE_SIZE;

    let t = SEGMENTS_BITSET.borrow();
    *SEGMENTS_BITSET.get_mut(&t) = BitSetRaw::new(NUM_PAGES as usize, kernel_end as *mut u64);


    // mark kernel and bitset pages as taken
    for i in 0..bitset_size_pages + kernel_size_pages {
        SEGMENTS_BITSET.get_mut(&t).set(i as usize, true);
    }

    let stack_size = NUM_CORES * STACK_SIZE;
    let stack_size_pages = (stack_size as u64).div_ceil(PAGE_SIZE);

    // mark stack pages as taken
    for i in 0..stack_size_pages {
        SEGMENTS_BITSET.get_mut(&t).set((NUM_PAGES - 1 - i) as usize, true);
    }
    SEGMENTS_BITSET.release(t);

    let page_table = create_page_table();
    for i in 0..3 {
        unsafe {
            *page_table.add(i) = create_page_table_entry(((i as u64) << (12 + 2 * 9)) as PhysAddr) | PTE_READ | PTE_WRITE | PTE_EXECUTE;
        }
    }
    switch_to_page_table(page_table);

    unsafe {
        KERNEL_PAGE_TABLE = page_table;
    }

    init_std_memory(&page_allocator, &page_deallocator, HEAP_TREE_ADDR, HEAP_BASE_ADDR, HEAP_ADDR);
}

pub fn init_paging_hart() {
    unsafe {
        switch_to_page_table(KERNEL_PAGE_TABLE);
    }
}

pub type PageTableEntry = u64;
pub type PageTable = *mut PageTableEntry;

pub const PTE_PRESENT: u64 = 1 << 0;
pub const PTE_READ: u64 = 1 << 1;
pub const PTE_WRITE: u64 = 1 << 2;
pub const PTE_EXECUTE: u64 = 1 << 3;
pub const PTE_USER: u64 = 1 << 4;

const PAGE_TABLE_SIZE: usize = 512;

static mut CURRENT_PAGE_TABLE: [PageTable; NUM_CORES] = [0 as PageTable; NUM_CORES];
static mut KERNEL_PAGE_TABLE: PageTable = 0 as PageTable;

pub fn create_page_table() -> PageTable {
    let page_table = alloc_page() as PageTable;
    unsafe {
        write_bytes(page_table as *mut u8, 0, PAGE_SIZE as usize);
    }

    unsafe {
        if KERNEL_PAGE_TABLE != 0 as PageTable {
            for i in 0..KERNEL_PT_ROOT_ENTRIES {
                *page_table.add(i as usize) = *KERNEL_PAGE_TABLE.add(i as usize);
            }
            for i in KERNEL_PT_ROOT_ENTRIES..PAGE_TABLE_SIZE as u64 {
                assert_eq!(0, *KERNEL_PAGE_TABLE.add(i as usize));
            }
        }
    }

    page_table
}

pub fn destroy_page_table(page_table: PageTable) {
    for i in 0..PAGE_TABLE_SIZE {
        let entry = *get_sub_page_table_entry(page_table, i);
        let kernel_entry = *get_sub_page_table_entry(unsafe { KERNEL_PAGE_TABLE }, i);
        if is_entry_table(entry) && entry != kernel_entry {
            destroy_page_table(get_entry_addr(entry).unwrap());
        }
    }
    free_page(page_table as PhysAddr);
}

pub fn switch_to_page_table(page_table: PageTable) {
    debug_assert_eq!(page_table as u64 % PAGE_SIZE, 0);
    fence(Ordering::Release);
    unsafe {
        if CURRENT_PAGE_TABLE[get_core_id() as usize] == page_table {
            return;
        }
        CURRENT_PAGE_TABLE[get_core_id() as usize] = page_table;
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
    let mut curr_table = unsafe { CURRENT_PAGE_TABLE[get_core_id() as usize] };
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

#[allow(clippy::fn_params_excessive_bools)]
pub fn map_page(virtual_addr: VirtAddr, physical_addr: PhysAddr, ignore_if_exists: bool, writable: bool, user: bool, executable: bool) {
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
    if executable {
        *curr_entry |= PTE_EXECUTE;
    }
    refresh_paging();
}

#[allow(clippy::fn_params_excessive_bools)]
pub fn map_page_auto(virtual_addr: VirtAddr, ignore_if_exists: bool, writable: bool, user: bool, executable: bool) {
    map_page(virtual_addr, alloc_page(), ignore_if_exists, writable, user, executable);
}

pub fn virt_to_phys(addr: VirtAddr) -> Option<PhysAddr> {
    if (addr as u64) < ID_MAP_END {
        return Some(addr as PhysAddr);
    }

    let entry = get_address_page_table_entry(addr).unwrap();
    Some(get_entry_addr(*entry)? as PhysAddr)
}

pub fn unmap_page(virtual_addr: VirtAddr) {
    let curr_entry = get_address_page_table_entry(virtual_addr).unwrap();
    debug_assert!((*curr_entry & PTE_PRESENT) == PTE_PRESENT);
    *curr_entry = 0;
    refresh_paging();
}
