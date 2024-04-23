use kernel_test::{kernel_test, kernel_test_mod};

use crate::memory::BitSetRaw;
use crate::rand::Rng;

#[cfg(feature = "run_tests")]
use super::get_free_space_addr;

kernel_test_mod!(crate::tests::A2_bitset);

#[kernel_test]
fn test_bitset_new() {
    let bitset = unsafe {
        let mut bitset = BitSetRaw::new(1024 * 8, get_free_space_addr() as *mut u64);
        bitset
    };
    for i in 0..1024 * 8 {
        assert!(!bitset.get(i));
    }
}

#[kernel_test]
fn test_bitset_set() {
    let mut bitset = unsafe {
        let mut bitset = BitSetRaw::new(1024 * 8, get_free_space_addr() as *mut u64);
        bitset.clear();
        bitset
    };

    let mut rng = Rng::new(5437891);

    let mut arr = [0; 1024 * 8];
    for i in 0..100000 {
        let idx = rng.get(0, 1024 * 8) as usize;
        assert_eq!(arr[idx] == 1, bitset.get(idx));
        bitset.set(idx, arr[idx] == 0);
        arr[idx] ^= 1;
        assert_eq!(arr[idx] == 1, bitset.get(idx));
    }
}

#[kernel_test]
fn test_bitset_zero_count() {
    let mut bitset = unsafe {
        let mut bitset = BitSetRaw::new(1024 * 8, get_free_space_addr() as *mut u64);
        bitset.clear();
        bitset
    };

    let mut rng = Rng::new(5437891);

    let mut arr = [0; 1024 * 8];
    let mut count0 = 1024 * 8;
    for i in 0..100000 {
        let idx = rng.get(0, 1024 * 8) as usize;
        assert_eq!(arr[idx] == 1, bitset.get(idx));
        bitset.set(idx, arr[idx] == 0);
        arr[idx] ^= 1;
        if arr[idx] == 0 {
            count0 += 1;
        } else {
            count0 -= 1;
        }
        assert_eq!(arr[idx] == 1, bitset.get(idx));
        assert_eq!(bitset.get_count0(), count0);
    }
}

#[kernel_test]
fn test_bitset_get_zero() {
    let mut bitset = unsafe {
        let mut bitset = BitSetRaw::new(1024 * 8, get_free_space_addr() as *mut u64);
        bitset.clear();
        bitset
    };

    let mut rng = Rng::new(5437891);

    let mut arr = [0; 1024 * 8];
    for i in 0..100000 {
        let idx = rng.get(0, 1024 * 8) as usize;
        assert_eq!(arr[idx] == 1, bitset.get(idx));
        bitset.set(idx, arr[idx] == 0);
        arr[idx] ^= 1;
        assert_eq!(arr[idx] == 1, bitset.get(idx));
        if bitset.get_count0() != 0 {
            assert!(!bitset.get(bitset.get_zero_element().unwrap()));
        }
    }
}
