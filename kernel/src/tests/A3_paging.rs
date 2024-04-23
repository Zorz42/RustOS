use kernel_test::{kernel_test, kernel_test_mod};

use crate::memory::{find_free_page, free_page, memset_int64, VIRTUAL_OFFSET};
use crate::rand::Rng;

kernel_test_mod!(crate::tests::A3_paging);

#[kernel_test]
fn test_one_page() {
    let _ = find_free_page();
}

#[kernel_test]
fn test_page_free() {
    let mut rng = Rng::new(54375893);

    let mut pages = [0 as *mut u8; 1024];
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
                memset_int64(pages[i].add(VIRTUAL_OFFSET as usize), 0, 4096);
            }
        }
        for i in 0..1024 {
            unsafe {
                free_page(pages[perm[i]]);
            }
        }
    }
}
