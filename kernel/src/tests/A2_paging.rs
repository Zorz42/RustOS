use core::ptr::write_bytes;
use kernel_test::{kernel_test, kernel_test_mod};

use crate::memory::map_page;
use crate::memory::{alloc_page, free_page, unmap_page, virt_to_phys, PhysAddr, VirtAddr, PAGE_SIZE, TESTING_OFFSET};
use kernel_std::Rng;

kernel_test_mod!(crate::tests::A2_paging);

#[kernel_test]
fn test_one_page() {
    let page = alloc_page();
    free_page(page);
}

#[kernel_test]
fn test_page_free() {
    let mut rng = Rng::new(54375893);

    let mut pages = [0 as PhysAddr; 1024];
    for _ in 0..20 {
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
            pages[i] = alloc_page();
            let val = rng.get(0, 1 << 8) as u8;
            unsafe {
                write_bytes((pages[i] as VirtAddr) as *mut u8, val, PAGE_SIZE as usize);
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

    let page_ptr = alloc_page() as u64;
    map_page(offset, page_ptr, false, true, false, false);
    unsafe {
        write_bytes(offset, 0, PAGE_SIZE as usize);
        free_page(page_ptr);
    }
    unmap_page(offset);
}

#[kernel_test]
fn test_page_write_stays() {
    const num_pages: usize = 200;
    let offset = TESTING_OFFSET as *mut u8;
    let offset_u64 = offset as *mut u64;
    let mut pages = [0; num_pages];

    for i in 0..num_pages {
        unsafe {
            pages[i] = alloc_page();
            map_page(offset.add(i * PAGE_SIZE as usize), pages[i], false, true, false, false);
        }
    }

    for i in 0..num_pages * PAGE_SIZE as usize / 8 {
        unsafe {
            *offset_u64.add(i as usize) = i as u64;
        }
    }

    for i in 0..num_pages * PAGE_SIZE as usize / 8 {
        unsafe {
            assert_eq!(*offset_u64.add(i), i as u64);
        }
    }

    for i in 0..num_pages {
        unsafe {
            free_page(pages[i]);
        }
    }

    for i in 0..num_pages {
        unsafe {
            unmap_page(offset.add(i * PAGE_SIZE as usize));
        }
    }
}
