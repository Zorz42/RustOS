use kernel_test::{kernel_test, kernel_test_mod};

use crate::memory::{memset, memset_int64};
use crate::rand::Rng;

kernel_test_mod!(crate::tests::A1_utils);

#[cfg(debug_assertions)]
static mut FREE_SPACE: [u8; 1032] = [0; 1032];

#[cfg(debug_assertions)]
fn get_free_space_addr() -> *mut u8 {
    unsafe { (FREE_SPACE.as_mut_ptr() as u64 / 8 * 8) as *mut u8 }
}

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
        let val = rng.get(0, (1 << 8) - 1) as u8;

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
