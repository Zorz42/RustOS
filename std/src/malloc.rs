use crate::heap_tree::HeapTree;
use crate::{allocate_page, HEAP_ADDR, HEAP_TREE_ADDR};

const PAGE_SIZE: u64 = 4096;

static mut CURR_PAGE: *mut u8 = 0 as *mut u8;
static mut HEAP_TREE: HeapTree = HeapTree::new_empty();

pub fn init_malloc() {
    unsafe {
        HEAP_TREE = HeapTree::new(HEAP_TREE_ADDR as *mut u8);
        CURR_PAGE = HEAP_ADDR as *mut u8;
    }
}

pub fn malloc(size: usize) -> *mut u8 {
    let mut actual_size = 8; // at least the size of u64
    let mut actual_size_log2 = 3;
    while actual_size < size {
        actual_size *= 2;
        actual_size_log2 += 1;
    }

    unsafe {
        let ptr = (HEAP_TREE.alloc(actual_size_log2 - 3) as u64 * 8 + HEAP_ADDR) as *mut u8;
        let ptr_end = ptr as u64 + actual_size as u64;

        while (CURR_PAGE as u64) < ptr_end {
            allocate_page(CURR_PAGE);
            CURR_PAGE = CURR_PAGE.add(PAGE_SIZE as usize);
        }

        ptr
    }
}

pub unsafe fn free(ptr: *mut u8) {
    unsafe {
        HEAP_TREE.free(((ptr as u64 - HEAP_ADDR) / 8) as u32);
    }
}
