use crate::heap_tree::HeapTree;
use crate::{allocate_page, Mutable, HEAP_ADDR, HEAP_TREE_ADDR};

const PAGE_SIZE: u64 = 4096;

static CURR_PAGE: Mutable<*mut u8> = Mutable::new(0 as *mut u8);
static HEAP_TREE: Mutable<HeapTree> = Mutable::new(HeapTree::new_empty());

pub fn init_malloc() {
    let t = HEAP_TREE.borrow();
    *HEAP_TREE.get_mut(&t) = unsafe { HeapTree::new(HEAP_TREE_ADDR as *mut u8) };

    HEAP_TREE.release(t);
    let t = CURR_PAGE.borrow();
    *CURR_PAGE.get_mut(&t) = unsafe { HEAP_ADDR as *mut u8 };
    CURR_PAGE.release(t);
}

pub fn malloc(size: usize) -> *mut u8 {
    let mut actual_size = 8; // at least the size of u64
    let mut actual_size_log2 = 3;
    while actual_size < size {
        actual_size *= 2;
        actual_size_log2 += 1;
    }

    unsafe {
        let t1 = HEAP_TREE.borrow();
        let t2 = CURR_PAGE.borrow();
        let ptr = (HEAP_TREE.get_mut(&t1).alloc(actual_size_log2 - 3) as u64 * 8 + HEAP_ADDR) as *mut u8;
        let ptr_end = ptr as u64 + actual_size as u64;

        while (*CURR_PAGE.get_mut(&t2) as u64) < ptr_end {
            allocate_page(*CURR_PAGE.get_mut(&t2), false);
            *CURR_PAGE.get_mut(&t2) = CURR_PAGE.get_mut(&t2).add(PAGE_SIZE as usize);
        }

        HEAP_TREE.release(t1);
        CURR_PAGE.release(t2);

        ptr
    }
}

pub unsafe fn free(ptr: *mut u8) {
    let t = HEAP_TREE.borrow();
    unsafe {
        HEAP_TREE.get_mut(&t).free(((ptr as u64 - HEAP_ADDR) / 8) as u32);
    }
    HEAP_TREE.release(t);
}
