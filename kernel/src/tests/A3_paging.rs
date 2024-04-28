use kernel_test::{kernel_test, kernel_test_mod};

use crate::memory::{find_free_page, free_page, map_page, map_page_auto, memset_int64, PhysAddr, VirtAddr, PAGE_SIZE, TESTING_OFFSET, VIRTUAL_OFFSET};
use crate::println;
use crate::rand::Rng;
use crate::print;

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
            let val = rng.get(0, 1u64 << 63);
            unsafe {
                memset_int64((pages[i] as VirtAddr).add(VIRTUAL_OFFSET as usize), val, 4096);
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
    let offset = TESTING_OFFSET as *mut u8;

    let page_ptr = find_free_page() as u64;
    map_page(offset, page_ptr, true, false);
    unsafe {
        memset_int64(offset, 0, PAGE_SIZE as usize);
        free_page(page_ptr);
    }
}

#[kernel_test]
fn test_page_write_stays() {
    const num_pages: usize = 1000;
    let offset = TESTING_OFFSET as *mut u8;
    let offset_u64 = offset as *mut u64;
    let mut pages = [0; num_pages];

    for i in 0..num_pages {
        unsafe {
            pages[i] = find_free_page();
            map_page(offset.add(i * PAGE_SIZE as usize), pages[i], true, false);
        }
    }

    for i in 0..num_pages * PAGE_SIZE as usize / 8 {
        unsafe {
            *offset_u64.add(i as usize) = i as u64;
        }
    }

    for i in 0..num_pages * PAGE_SIZE as usize / 8 {
        unsafe {
            if *offset_u64.add(i) != i as u64 {
                println!("Mismatch at position {} 0x{:x}", i, i);
            }
            assert_eq!(*offset_u64.add(i), i as u64);
        }
    }

    for i in 0..num_pages {
        unsafe {
            free_page(pages[i]);
        }
    }
}
