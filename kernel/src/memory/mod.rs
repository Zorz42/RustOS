mod bitset;
mod paging;

pub const MEMORY_SIZE: u64 = 128 * 1024 * 1024;

pub const PAGE_SIZE: u64 = 4096;
pub const KERNEL_OFFSET: u64 = 0x80000000;
pub const NUM_PAGES: u64 = MEMORY_SIZE / PAGE_SIZE;

pub const KERNEL_PT_ROOT_ENTRIES: u64 = 13; // how many entries are used for kernel page table (all the rest is for user page tables)

pub const ID_MAP_END: u64 = 3u64 << 30; // the end of identity mapping of physical memory
const FRAME_SIZE: u64 = 1u64 << 30;
// where the heap starts
pub const HEAP_BASE_ADDR: u64 = ID_MAP_END;
// where the heap tree starts (describes the heap)
pub const HEAP_TREE_ADDR: u64 = ID_MAP_END + FRAME_SIZE;
// for testing purposes
#[allow(dead_code)]
pub const TESTING_OFFSET: u64 = ID_MAP_END + 2 * FRAME_SIZE;
// where users program stack lives
pub const USER_STACK: u64 = ID_MAP_END + 3 * FRAME_SIZE;
// where the user context is stored (registers have to be saved when user program is interrupted)
pub const USER_CONTEXT: u64 = ID_MAP_END + 4 * FRAME_SIZE;
// This is where the heap starts. It is split into 20 regions, each with size 1 << 28 bytes
pub const HEAP_ADDR: u64 = ID_MAP_END + 5 * FRAME_SIZE;
// this is the top of used kernel virtual memory space
#[allow(dead_code)]
pub const KERNEL_VIRTUAL_END: u64 = HEAP_ADDR + 20 * HEAP_REGION_SIZE;

// this marks the end of kernel virtual memory space (taken by first KERNEL_PT_ROOT_ENTRIES entries)
pub const KERNEL_VIRTUAL_TOP: u64 = KERNEL_PT_ROOT_ENTRIES * (1u64 << 30);

// statically assert that the kernel fits into the virtual memory
const _: [(); (KERNEL_VIRTUAL_TOP - KERNEL_VIRTUAL_END) as usize] = [(); (KERNEL_VIRTUAL_TOP - KERNEL_VIRTUAL_END) as usize];

use kernel_std::HEAP_REGION_SIZE;
pub use paging::{alloc_page, destroy_page_table, alloc_continuous_pages, free_page, get_num_free_pages, init_paging, init_paging_hart, map_page, map_page_auto, unmap_page, virt_to_phys, PhysAddr, VirtAddr, PageTable, create_page_table, switch_to_page_table};

extern "C" {
    pub static _end: u8;
}

pub fn get_kernel_top_address() -> u64 {
    unsafe { &_end as *const u8 as u64 + 30 * PAGE_SIZE }
}
