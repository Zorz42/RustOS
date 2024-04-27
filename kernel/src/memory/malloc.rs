use crate::memory::{HEAP_BASE, map_page_auto, PAGE_SIZE};

static mut CURR_PTR: *mut u8 = HEAP_BASE as *mut u8;
static mut CURR_PAGE: *mut u8 = HEAP_BASE as *mut u8;

pub fn malloc(size: usize) -> *mut u8 {
    unsafe {
        let result = CURR_PTR;
        CURR_PTR = CURR_PTR.add(size);
        while (CURR_PAGE as u64) < (CURR_PTR as u64) {
            map_page_auto(CURR_PAGE, true, false);
            CURR_PAGE = CURR_PAGE.add(PAGE_SIZE as usize);
        }
        result
    }
}

pub unsafe fn free(ptr: *mut u8) {
    // ignore for now
}
