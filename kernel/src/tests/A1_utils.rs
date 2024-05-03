use kernel_test::{kernel_test, kernel_test_mod};

#[cfg(feature = "run_tests")]
use std::{memcpy, memcpy_non_aligned, memset, memset_int64, Rng};
use std::swap;

#[cfg(feature = "run_tests")]
use super::get_free_space_addr;

kernel_test_mod!(crate::tests::A1_utils);

#[kernel_test]
fn test_memset_u64() {
    let mut rng = Rng::new(54375839);
    for _ in 0..1000 {
        let offset = rng.get(0, 1024 / 8);
        let len = rng.get(0, 1024 / 8 - offset);
        let val = rng.get(0, (1u64 << 63) - 1 + (1u64 << 63));

        unsafe {
            memset_int64(get_free_space_addr().add((8 * offset) as usize), val, (len * 8) as usize);
            for i in 0..len {
                assert_eq!(*(get_free_space_addr() as *mut u64).add((offset + i) as usize), val);
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
            memset(get_free_space_addr().add(offset as usize), val, len as usize);
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
            memcpy(free_addr.add(offset1 as usize) as *mut u8, free_addr.add(offset2 as usize) as *mut u8, len as usize * 8);
        }
        for i in 0..len {
            unsafe {
                assert_eq!(arr[i as usize], *free_addr.add((offset1 + i) as usize));
                assert_eq!(*free_addr.add((offset1 + i) as usize), *free_addr.add((offset2 + i) as usize))
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
            memcpy_non_aligned(free_addr.add(offset1 as usize), free_addr.add(offset2 as usize), len as usize);
        }

        for i in 0..len {
            unsafe {
                assert_eq!(arr[i as usize], *free_addr.add((offset1 + i) as usize));
                assert_eq!(*free_addr.add((offset1 + i) as usize), *free_addr.add((offset2 + i) as usize))
            }
        }
    }
}

#[kernel_test]
fn test_memset_u64_exact_bounds() {
    let mut rng = Rng::new(53452534);
    let free_addr = get_free_space_addr() as *mut u64;

    for _ in 0..1000 {
        let a1 = rng.get(0, 1024 / 8);
        let a2 = rng.get(0, 1024 / 8);
        let x1 = u64::min(a1, a2);
        let x2 = u64::max(a1, a2);
        let val1 = rng.get(0, 1u64 << 63);
        let val2 = rng.get(0, 1u64 << 63);

        unsafe {
            memset_int64(free_addr as *mut u8, val1, 1024);
            memset_int64(free_addr.add(x1 as usize) as *mut u8, val2, (x2 - x1) as usize * 8);
        }

        for i in 0..1024 / 8 {
            let val = if x1 <= i && i < x2 { val2 } else { val1 };

            unsafe {
                assert_eq!(*free_addr.add(i as usize), val);
            }
        }
    }
}

#[kernel_test]
fn test_memset_exact_bounds() {
    let mut rng = Rng::new(647538);
    let free_addr = get_free_space_addr();

    for _ in 0..1000 {
        let a1 = rng.get(0, 1024);
        let a2 = rng.get(0, 1024);
        let x1 = u64::min(a1, a2);
        let x2 = u64::max(a1, a2);
        let val1 = rng.get(0, 256) as u8;
        let val2 = rng.get(0, 256) as u8;

        unsafe {
            memset(free_addr, val1, 1024);
            memset(free_addr.add(x1 as usize), val2, (x2 - x1) as usize);
        }

        for i in 0..1024 {
            let val = if x1 <= i && i < x2 { val2 } else { val1 };

            unsafe {
                assert_eq!(*free_addr.add(i as usize), val);
            }
        }
    }
}

#[kernel_test]
fn test_memcpy_u64_exact_bounds() {
    let mut rng = Rng::new(76253462);
    let free_addr = get_free_space_addr() as *mut u64;

    for t in 0..1000 {
        let a1 = rng.get(0, 256 / 8);
        let a2 = rng.get(0, 256 / 8);
        let x1 = u64::min(a1, a2);
        let x2 = u64::max(a1, a2);

        for i in 0..512 / 8 {
            unsafe {
                *free_addr.add(i) = rng.get(0, 1u64 << 63);
            }
        }

        unsafe {
            memcpy(free_addr as *mut u8, free_addr.add(512 / 8) as *mut u8, 256);
            memcpy(free_addr.add(256 / 8 + x1 as usize) as *mut u8, free_addr.add(512 / 8 + x1 as usize) as *mut u8, (x2 - x1) as usize * 8);
        }

        for i in 0..256 / 8 {
            let val = if x1 <= i && i < x2 {
                unsafe { *free_addr.add((256 / 8 + i) as usize) }
            } else {
                unsafe { *free_addr.add(i as usize) }
            };

            unsafe {
                assert_eq!(*free_addr.add((512 / 8 + i) as usize), val);
            }
        }
    }
}

#[kernel_test]
fn test_memcpy_exact_bounds() {
    let mut rng = Rng::new(23456789);
    let free_addr = get_free_space_addr();

    for t in 0..1000 {
        let a1 = rng.get(0, 256);
        let a2 = rng.get(0, 256);
        let x1 = u64::min(a1, a2);
        let x2 = u64::max(a1, a2);

        for i in 0..512 {
            unsafe {
                *free_addr.add(i) = rng.get(0, 256) as u8;
            }
        }

        unsafe {
            memcpy_non_aligned(free_addr, free_addr.add(512), 256);
            memcpy_non_aligned(free_addr.add(256 + x1 as usize), free_addr.add(512 + x1 as usize), (x2 - x1) as usize);
        }

        for i in 0..256 {
            let val = if x1 <= i && i < x2 {
                unsafe { *free_addr.add((256 + i) as usize) }
            } else {
                unsafe { *free_addr.add(i as usize) }
            };

            unsafe {
                assert_eq!(*free_addr.add((512 + i) as usize), val);
            }
        }
    }
}

#[kernel_test]
fn test_swap() {
    let mut val1 = 67862423u64;
    let mut val2 = 43262436u64;
    swap(&mut val1, &mut val2);
    assert_eq!(val1, 43262436u64);
    assert_eq!(val2, 67862423u64);
}