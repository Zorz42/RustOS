use kernel_test::{kernel_test, kernel_test_mod};

use crate::memory::{find_free_page, free_page};

kernel_test_mod!(crate::tests::A3_paging);

#[kernel_test(crate::tests::A3_paging)]
fn test_one_page() {
    let _ = find_free_page();
}

#[kernel_test(crate::tests::A3_paging)]
fn test_one_page_free() {
    let page = find_free_page();
    unsafe {
        free_page(page);
    }
}
