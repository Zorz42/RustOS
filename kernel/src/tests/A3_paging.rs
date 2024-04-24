use kernel_test::{kernel_test, kernel_test_mod};

use crate::memory::{find_free_page, free_page, map_page, memset_int64, VIRTUAL_OFFSET, PhysAddr, VirtAddr};
use crate::rand::Rng;

kernel_test_mod!(crate::tests::A3_paging);

#[kernel_test]
fn test_one_page() {
    let _ = find_free_page();
}

#[kernel_test]
fn test_page_free() {
    let mut rng = Rng::new(54375893);

    let mut pages = [0 as PhysAddr; 1024];
    for _ in 0..100 {
        // create a random permutation
        let mut perm = [0; 1024];
        for i in 0..1024 {
            perm[i] = i;
        }
        for i1 in 0..1024 {
            let i2 = rng.get(0, 1024) as usize;
            let temp = perm[i1];
            perm[i1] = perm[i2];
            perm[i2] = temp;
        }

        for i in 0..1024 {
            pages[i] = find_free_page();
            unsafe {
                memset_int64((pages[i] as VirtAddr).add(VIRTUAL_OFFSET as usize), 0, 4096);
            }
        }
        for i in 0..1024 {
            unsafe {
                free_page(pages[perm[i]]);
            }
        }
    }
}

#[kernel_test]
fn test_page_write() {
    let offset = (1u64 << 44) as *mut u8;

    let page_ptr = find_free_page() as u64;
    map_page(offset, page_ptr, true, false);
    unsafe {
        memset_int64(offset, 0, 4096);
        free_page(page_ptr);
    }
}
