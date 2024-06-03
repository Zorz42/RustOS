mod bitset;
mod paging;

pub const MEMORY_SIZE: u64 = 128 * 1024 * 1024;

pub const PAGE_SIZE: u64 = 4096;
pub const KERNEL_OFFSET: u64 = 0x80000000;
pub const TOP_ADDR: u64 = KERNEL_OFFSET + MEMORY_SIZE;
pub const NUM_PAGES: u64 = MEMORY_SIZE / PAGE_SIZE;


pub const ID_MAP_END: u64 = 3u64 << (12 + 2 * 9); // the end of identity mapping of physical memory
const FRAME_SIZE: u64 = 1u64 << 30;
pub const HEAP_BASE_ADDR: u64 = ID_MAP_END;
pub const HEAP_TREE_ADDR: u64 = ID_MAP_END + FRAME_SIZE;
#[allow(dead_code)]
pub const TESTING_OFFSET: u64 = ID_MAP_END + 2 * FRAME_SIZE;
pub const FRAMEBUFFER_OFFSET: u64 = ID_MAP_END + 3 * FRAME_SIZE;
pub const DISK_OFFSET: u64 = ID_MAP_END + 4 * FRAME_SIZE;

pub use paging::{init_paging, get_num_free_pages, free_page, alloc_page, VirtAddr, PhysAddr, map_page, map_page_auto, unmap_page, virt_to_phys, init_paging_hart};
pub use bitset::{BitSetRaw, bitset_size_bytes};

extern "C" {
    pub static _end: u8;
}

pub fn get_kernel_top_address() -> u64 {
    unsafe { &_end as *const u8 as u64 }
}

