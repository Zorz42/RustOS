#![no_std]
#![feature(decl_macro)]

mod heap_tree;
mod malloc;
mod utils;
mod boxed;
mod rand;
mod pointer;
mod vector;

#[cfg(feature = "test_includes")]
pub use heap_tree::HeapTree;
#[cfg(feature = "test_includes")]
pub use malloc::{malloc, free};

pub use utils::{memcpy, memcpy_non_aligned, memset, memset_int64, volatile_store_byte, addr_of, swap};
pub use rand::Rng;
pub use boxed::Box;
pub use vector::Vec;
use crate::malloc::init_malloc;

static mut PAGE_ALLOCATOR: Option<&'static dyn Fn(*mut u8)> = None;
static mut HEAP_TREE_ADDR: u64 = 0;
static mut HEAP_ADDR: u64 = 0;

fn allocate_page(page: *mut u8) {
    unsafe {
        if let Some(page_allocator) = PAGE_ALLOCATOR {
            page_allocator(page);
        } else {
            panic!("Std library memory was not initialized!");
        }
    }
}

pub fn init_std_memory(page_allocator: &'static dyn Fn(*mut u8), heap_tree_addr: u64, heap_addr: u64) {
    unsafe {
        PAGE_ALLOCATOR = Some(page_allocator);
        HEAP_TREE_ADDR = heap_tree_addr;
        HEAP_ADDR = heap_addr;
    }
    init_malloc();
}