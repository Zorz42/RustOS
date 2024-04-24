use kernel_test::{kernel_test, kernel_test_mod};

use crate::memory::{free, malloc, memset};
use crate::rand::Rng;

kernel_test_mod!(crate::tests::A4_malloc);

#[kernel_test]
fn test_malloc() {
    let mut rng = Rng::new(754389);
    let _ = malloc(0);
    for _ in 0..100 {
        let _ = malloc(rng.get(0, 100) as usize);
    }
}

#[kernel_test]
fn test_malloc_free() {
    let mut rng = Rng::new(5674382);

    let mut ptrs = [0 as *mut u8; 1024];
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
            let len = rng.get(0, 100);
            ptrs[i] = malloc(len as usize);
            unsafe {
                memset(ptrs[i], 12, len as usize);
            }
        }
        for i in 0..1024 {
            unsafe {
                free(ptrs[perm[i]]);
            }
        }
    }
}

#[kernel_test]
fn test_malloc_write_stays() {
    let mut rng = Rng::new(745421);

    let ptr = malloc(0);
    let _ = malloc((4 - (ptr as u64) % 4) as usize);

    let mut ptrs = [0 as *mut u8; 1024];
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

        let mut arr = [0; 1024];
        for i in 0..1024 {
            arr[i] = rng.get(0, 1u64<<32);
        }

        for i in 0..1024 {
            let len = 4;
            ptrs[perm[i]] = malloc(len as usize);
            unsafe {
                *(ptrs[perm[i]] as *mut u32) = arr[i] as u32;
            }
        }

        for i in 0..1024 {
            unsafe {
                assert_eq!(*(ptrs[perm[i]] as *mut u32), arr[i] as u32);
            }
        }

        for i in 0..1024 {
            unsafe {
                free(ptrs[i]);
            }
        }
    }
}

#[kernel_test]
fn test_malloc_free_works() {
    let mut rng = Rng::new(657438);
    for i in 0..100000 {
        let ptr = malloc(rng.get(0, 0x1000) as usize);
        unsafe {
            free(ptr);
        }
    }
}