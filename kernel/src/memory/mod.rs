use core::arch::asm;

use bootloader_api::info::{MemoryRegionKind, MemoryRegions};

pub use bitset::BitSetRaw;
pub use paging::{find_free_page, free_page, map_page};
use paging::{PageTable, SEGMENTS_BITSET};
pub use utils::*;

use crate::memory::paging::CURRENT_PAGE_TABLE;

mod bitset;
mod paging;
mod utils;

pub const PAGE_SIZE: u64 = 4096;
pub const VIRTUAL_OFFSET: u64 = 0x100000000;
pub const KERNEL_STACK_SIZE: u64 = 100 * 1024; // 100 KiB
pub const KERNEL_STACK_ADDR: u64 = 0x200000000 - KERNEL_STACK_SIZE;

pub fn init_memory(memory_regions: &MemoryRegions) {
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
            if region.kind != MemoryRegionKind::Usable || region.start == 0 {
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
        SEGMENTS_BITSET = BitSetRaw::new(
            num_all_pages as usize,
            (VIRTUAL_OFFSET + bitset_addr) as *mut u64,
        );
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
        if region.kind == MemoryRegionKind::Usable && region.start != 0 {
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
        map_page(
            page * PAGE_SIZE + VIRTUAL_OFFSET,
            page * PAGE_SIZE,
            true,
            false,
        );
    }
}
