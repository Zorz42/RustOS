use kernel_test::{kernel_test, kernel_test_mod};

use crate::memory::{memcpy, memcpy_non_aligned, memset, memset_int64};
use crate::rand::Rng;

#[cfg(feature = "run_tests")]
use super::get_free_space_addr;

kernel_test_mod!(crate::tests::A1_utils);

#[kernel_test]
fn test_memset_u64() {
    let mut rng = Rng::new(54375839);
    for _ in 0..1000 {
        let offset = rng.get(0, 1024 / 8);
        let len = rng.get(0, 1024 / 8 - offset);
        let val = rng.get(0, (1 << 63) - 1 + (1 << 63));

        unsafe {
            memset_int64(
                get_free_space_addr().add((8 * offset) as usize),
                val,
                (len * 8) as usize,
            );
            for i in 0..len {
                assert_eq!(
                    *(get_free_space_addr() as *mut u64).add((offset + i) as usize),
                    val
                );
            }
        }
    }
}

#[kernel_test]
fn test_memset() {
    let mut rng = Rng::new(6547382);
    for _ in 0..1000 {
        let offset = rng.get(0, 1024);
        let len = rng.get(0, 1024 - offset);
        let val = rng.get(0, 1 << 8) as u8;

        unsafe {
            memset(
                get_free_space_addr().add(offset as usize),
                val,
                len as usize,
            );
            for i in 0..len {
                assert_eq!(*get_free_space_addr().add((offset + i) as usize), val);
            }
        }
    }
}

#[kernel_test]
fn test_memcpy() {
    let free_addr = get_free_space_addr() as *mut u64;
    let mut rng = Rng::new(56437892);
    for _ in 0..1000 {
        let offset1 = rng.get(0, 1024 / 8 - 1);
        let len = rng.get(0, (1024 / 8 - offset1) / 2 + 1);
        let offset2 = rng.get(offset1 + len, 1024 / 8 - len + 1);
        let mut arr = [0 as u64; 1024 / 8];
        for i in 0..len {
            arr[i as usize] = rng.get(0, (1 << 63) - 1 + (1 << 63));
            unsafe {
                *free_addr.add((offset1 + i) as usize) = arr[i as usize];
            }
        }
        unsafe {
            memcpy(
                free_addr.add(offset1 as usize) as *mut u8,
                free_addr.add(offset2 as usize) as *mut u8,
                len as usize * 8,
            );
        }
        for i in 0..len {
            unsafe {
                assert_eq!(arr[i as usize], *free_addr.add((offset1 + i) as usize));
                assert_eq!(
                    *free_addr.add((offset1 + i) as usize),
                    *free_addr.add((offset2 + i) as usize)
                )
            }
        }
    }
}

#[kernel_test]
fn test_memcpy_non_aligned() {
    let free_addr = get_free_space_addr();
    let mut rng = Rng::new(7543212);
    for _ in 0..1000 {
        let offset1 = rng.get(0, 1024 - 1);
        let len = rng.get(0, (1024 - offset1) / 2 + 1);
        let offset2 = rng.get(offset1 + len, 1024 - len + 1);
        let mut arr = [0 as u8; 1024];
        for i in 0..len {
            arr[i as usize] = rng.get(0, 1 << 8) as u8;
            unsafe {
                *free_addr.add((offset1 + i) as usize) = arr[i as usize];
            }
        }
        unsafe {
            memcpy_non_aligned(
                free_addr.add(offset1 as usize),
                free_addr.add(offset2 as usize),
                len as usize,
            );
        }

        for i in 0..len {
            unsafe {
                assert_eq!(arr[i as usize], *free_addr.add((offset1 + i) as usize));
                assert_eq!(
                    *free_addr.add((offset1 + i) as usize),
                    *free_addr.add((offset2 + i) as usize)
                )
            }
        }
    }
}
