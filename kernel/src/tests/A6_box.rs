use kernel_test::{kernel_test, kernel_test_mod};
use std::{Box, Rng};
use core::mem::MaybeUninit;

kernel_test_mod!(crate::tests::A6_box);

#[kernel_test]
fn test_box_new() {
    let mut rng = Rng::new(234567890987654);
    for _ in 0..1000 {
        let _ = Box::new(rng.get(0, 1u64 << 63));
    }
}

struct BigStruct {
    val: [u64; 100000],
}

#[kernel_test]
fn test_box_no_leak() {
    let mut rng = Rng::new(234567890987654);
    for _ in 0..10000 {
        let _ = Box::new(BigStruct { val: unsafe { MaybeUninit::uninit().assume_init() } });
    }
}