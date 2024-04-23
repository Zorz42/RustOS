use kernel_test::{kernel_test, kernel_test_mod};

use crate::memory::{memset, memset_int64};

kernel_test_mod!(crate::tests::A1_utils);

#[cfg(debug_assertions)]
static mut FREE_SPACE: [u8; 1032] = [0; 1032];

#[cfg(debug_assertions)]
fn get_free_space_addr() -> *mut u8 {
    unsafe { (FREE_SPACE.as_mut_ptr() as u64 / 8 * 8) as *mut u8 }
}

#[kernel_test]
fn test_memset_u64() {
    let tests = [
        (54278593275892752, 1024 / 8),
        (635334567986743589, 1024 / 8),
        (0, 1024 / 8),
        (12, 43),
        (564378643758436, 50),
        (433, 0),
        (0, 0),
    ];

    for (val, len) in tests {
        unsafe {
            memset_int64(get_free_space_addr(), val, len * 8);
            for i in 0..len {
                assert_eq!(*(get_free_space_addr() as *mut u64).add(i), val);
            }
        }
    }
}

#[kernel_test]
fn test_memset() {
    let tests = [
        (0, 164, 1024),
        (0, 63, 1024),
        (0, 0, 1024),
        (1, 12, 43),
        (2, 84, 50),
        (3, 12, 43),
        (4, 84, 50),
        (3, 32, 0),
        (0, 0, 0),
        (3, 0, 0),
    ];

    for (offset, val, len) in tests {
        unsafe {
            memset(get_free_space_addr().add(offset), val, len);
            for i in 0..len {
                assert_eq!(*get_free_space_addr().add(offset + i), val);
            }
        }
    }
}
