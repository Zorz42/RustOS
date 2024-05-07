use kernel_test::{kernel_test, kernel_test_mod};
use std::{Box, Rng};

kernel_test_mod!(crate::tests::A6_box);

#[kernel_test]
fn test_box_new() {
    let mut rng = Rng::new(234567890987654);
    for _ in 0..1000 {
        let _ = Box::new(rng.get(0, 1u64 << 63));
    }
}

// to be honest, this is more of a test to see if the code compiles
#[kernel_test]
fn test_box_deref() {
    struct SomeStruct {
        pub val1: i32,
        pub val2: i32,
        pub val3: char,
    }

    let b = Box::new(SomeStruct { val1: 42, val2: 1040, val3: 'A' });
    assert_eq!(b.val1, 42);
    assert_eq!(b.val2, 1040);
    assert_eq!(b.val3, 'A');
}

#[kernel_test]
fn test_box_keeps_value() {
    let mut rng = Rng::new(65437296178543);
    for _ in 0..10 {
        const NONE: Option<Box<u64>> = None;

        let mut arr = [0; 512];
        let mut box_arr = [NONE; 512];

        for i in 0..512 {
            arr[i] = rng.get(0, 1u64 << 63);
            box_arr[i] = Some(Box::new(arr[i]));
        }

        for i in 0..512 {
            assert_eq!(Some(Box::new(arr[i])), box_arr[i]);
        }
    }
}

static mut DROP_COUNTER: i32 = 0;

struct DroppableStruct {}

impl Drop for DroppableStruct {
    fn drop(&mut self) {
        unsafe {
            DROP_COUNTER += 1;
        }
    }
}

#[kernel_test]
fn test_box_calls_drop() {
    let b = Box::new(DroppableStruct {});
    drop(b);

    unsafe {
        assert_eq!(DROP_COUNTER, 1);
        DROP_COUNTER = 0;
    }
}
