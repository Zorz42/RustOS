use kernel_test::{kernel_test, kernel_test_mod};

use crate::memory::BitSetRaw;

kernel_test_mod!(crate::tests::A2_bitset);

#[cfg(debug_assertions)]
static mut FREE_SPACE: [u8; 1032] = [0; 1032];

#[cfg(debug_assertions)]
fn get_free_space_addr() -> *mut u8 {
    unsafe { (FREE_SPACE.as_mut_ptr() as u64 / 8 * 8) as *mut u8 }
}

#[kernel_test]
fn test_bitset_clear() {
    unsafe {
        let mut bitset = BitSetRaw::new(1024 / 8, get_free_space_addr() as *mut u64);
        bitset.clear();
    }
}
