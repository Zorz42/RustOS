use kernel_test::{kernel_test, kernel_test_mod};

use super::get_free_space_addr;
use crate::memory::BitSetRaw;
use std::Rng;

kernel_test_mod!(crate::tests::A1_bitset);

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
            let idx = bitset.get_zero_element().unwrap();
            assert!(!bitset.get(idx));
        }
    }
}

#[kernel_test]
fn test_bitset_fill() {
    let mut bitset = unsafe {
        let mut bitset = BitSetRaw::new(1024 * 8, get_free_space_addr() as *mut u64);
        bitset
    };

    let mut arr = [0; 1024 * 8];
    for i in 0..1024 * 8 {
        let idx = bitset.get_zero_element().unwrap();
        assert!(arr[idx] == 0);
        assert!(!bitset.get(idx));
        bitset.set(idx, true);
        arr[idx] = 1;
        assert!(bitset.get(idx));
        assert_eq!(bitset.get_count0(), 1024 * 8 - i - 1);
    }
}

#[kernel_test]
fn test_bitset_store_load() {
    let mut bitset = unsafe {
        let mut bitset = BitSetRaw::new(1024, get_free_space_addr().add(1024) as *mut u64);
        bitset
    };

    let mut rng = Rng::new(5437891);

    let mut arr = [0; 1024];
    for i in 0..100 {
        let idx = rng.get(0, 1024) as usize;
        assert_eq!(arr[idx] == 1, bitset.get(idx));
        bitset.set(idx, arr[idx] == 0);
        arr[idx] ^= 1;
        unsafe {
            bitset.store_to(get_free_space_addr() as *mut u64);
            bitset.load_from(get_free_space_addr() as *mut u64);
        }
        assert_eq!(arr[idx] == 1, bitset.get(idx));
    }
}
