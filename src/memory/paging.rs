pub type PhysAddr = u64;
pub type VirtAddr = *mut u8;

use core::ptr::addr_of;
use crate::memory::bitset::{bitset_size_bytes, BitSetRaw};
use crate::memory::{get_kernel_top_address, KERNEL_OFFSET, NUM_PAGES, PAGE_SIZE};
use crate::println;

pub static mut SEGMENTS_BITSET: BitSetRaw = BitSetRaw::new_empty();

pub fn get_num_free_pages() -> u64 {
    unsafe {
        SEGMENTS_BITSET.get_count0() as u64
    }
}

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

pub fn init_paging() {
    let mut kernel_end = (get_kernel_top_address() + 2 * PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;
    let bitset_size_bytes = bitset_size_bytes(NUM_PAGES as usize);
    let bitset_size_pages = (bitset_size_bytes as u64 + PAGE_SIZE - 1) / PAGE_SIZE;
    let kernel_size_pages = (kernel_end - KERNEL_OFFSET) / PAGE_SIZE;

    unsafe {
        println!("0x{:x} 0x{:x}", get_kernel_top_address(), kernel_end);
    }

    unsafe {
        SEGMENTS_BITSET = BitSetRaw::new(NUM_PAGES as usize, kernel_end as *mut u64);
    }

    for i in 0..bitset_size_pages + kernel_size_pages {
        unsafe {
            SEGMENTS_BITSET.set(i as usize, true);
        }
    }
}