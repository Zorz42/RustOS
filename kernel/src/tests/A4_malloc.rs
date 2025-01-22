use core::ptr::write_bytes;
use kernel_test::{kernel_test, kernel_test_mod};
use kernel_std::{free, malloc, Rng, malloc2, free2};

kernel_test_mod!(crate::tests::A4_malloc);

#[kernel_test]
fn test_malloc() {
    let mut rng = Rng::new(754389);
    let _ = malloc2(0);
    for _ in 0..10000 {
        let _ = malloc2(rng.get(0, 100) as usize);
    }
}

#[kernel_test]
fn test_malloc_free() {
    let mut rng = Rng::new(5674382);

    let mut ptrs = [0 as *mut u8; 1024];
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
            let len = rng.get(0, 100);
            ptrs[i] = malloc2(len as usize);
            unsafe {
                write_bytes(ptrs[i], 12, len as usize);
            }
        }
        for i in 0..1024 {
            unsafe {
                free2(ptrs[perm[i]]);
            }
        }
    }
}

#[kernel_test]
fn test_malloc_write_stays() {
    let mut rng = Rng::new(745421);

    let ptr = malloc2(0);
    let _ = malloc2((4 - (ptr as u64) % 4) as usize);

    let mut ptrs = [0 as *mut u8; 1024];
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

        let mut arr = [0; 1024];
        for i in 0..1024 {
            arr[i] = rng.get(0, 1u64 << 32);
        }

        for i in 0..1024 {
            let len = 4;
            ptrs[perm[i]] = malloc2(len as usize);
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
                free2(ptrs[i]);
            }
        }
    }
}

#[kernel_test]
fn test_malloc_big() {
    let mut rng = Rng::new(657438);
    for i in 0..10000 {
        let ptr = malloc2(rng.get(0, 0x100000) as usize);
        unsafe {
            free2(ptr);
        }
    }
}

#[kernel_test]
fn test_big_malloc_write_stays() {
    let mut rng = Rng::new(745421);

    let ptr = malloc2(0);
    let _ = malloc2((4 - (ptr as u64) % 4) as usize);

    let mut ptrs = [0 as *mut u8; 1024];
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

        let mut arr = [0; 1024];
        for i in 0..1024 {
            arr[i] = rng.get(0, 1u64 << 32);
        }

        for i in 0..1024 {
            let len = rng.get(4, 1 << 16);
            ptrs[perm[i]] = malloc2(len as usize);
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
                free2(ptrs[i]);
            }
        }
    }
}