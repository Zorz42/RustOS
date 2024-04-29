use crate::memory::{map_page_auto, HeapTree, HEAP_BASE_ADDR, HEAP_TREE_ADDR, PAGE_SIZE};

static mut CURR_PAGE: *mut u8 = HEAP_BASE_ADDR as *mut u8;
static mut HEAP_TREE: HeapTree = HeapTree::new_empty();

pub fn init_malloc() {
    unsafe {
        HEAP_TREE = unsafe { HeapTree::new(HEAP_TREE_ADDR as *mut u8) };
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
        let ptr = (HEAP_TREE.alloc(actual_size_log2 - 3) as u64 * 8 + HEAP_BASE_ADDR) as *mut u8;
        let ptr_end = ptr as u64 + actual_size as u64;

        while (CURR_PAGE as u64) < ptr_end {
            map_page_auto(CURR_PAGE, true, false);
            CURR_PAGE = CURR_PAGE.add(PAGE_SIZE as usize);
        }

        return ptr;
    }
}

pub unsafe fn free(ptr: *mut u8) {
    unsafe {
        HEAP_TREE.free(((ptr as u64 - HEAP_BASE_ADDR) / 8) as u32);
    }
}
