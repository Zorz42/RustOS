use kernel_test::{kernel_test, kernel_test_mod};
use std::{Box, Rng, Vec};
kernel_test_mod!(crate::tests::A7_vector);


#[kernel_test]
fn test_vector_new() {
    for _ in 0..1000 {
        let vec = Vec::<i32>::new();
        assert_eq!(vec.size(), 0);
    }
}

#[kernel_test]
fn test_vector_new_with_size() {
    for i in 0..1000 {
        let vec = Vec::<i32>::new_with_size(i);
        assert_eq!(vec.size(), i);
    }
}

#[kernel_test]
fn test_vector_push_and_index() {
    let mut arr = [0; 1024];
    let mut vec = Vec::new();

    let mut rng = Rng::new(543758924052);

    for i in 0..1024 {
        arr[i] = rng.get(0, 1 << 32) as u32;
        vec.push(arr[i]);

        for j in 0..i + 1 {
            assert_eq!(arr[j], vec[j]);
        }
    }
}

#[kernel_test]
fn test_vector_pop() {
    let mut arr = [0; 1024];
    let mut arr_size = 0;
    let mut vec = Vec::new();

    let mut rng = Rng::new(543758924052);

    for i in 0..1024 {
        if rng.get(0, 2) == 0 && vec.size() != 0 {
            vec.pop();
            arr_size -= 1;
        } else {
            arr[arr_size] = rng.get(0, 1 << 32) as u32;
            vec.push(arr[arr_size]);
            arr_size += 1
        }

        assert_eq!(arr_size, vec.size());
        for j in 0..vec.size() {
            assert_eq!(arr[j], vec[j]);
        }
    }
}

#[kernel_test]
fn test_vector_index_out_of_bounds() {
    let mut rng = Rng::new(62346234);
    let mut arr = [0; 1024];
    for _ in 0..100 {
        let size = rng.get(0, 1024) as usize;
        let mut vec = Vec::new();
        for i in 0..size {
            arr[i] = rng.get(0, 1u64 << 63);
            vec.push(arr[i]);
        }

        for _ in 0..10000 {
            let idx = rng.get(0, 5000) as usize;
            assert_eq!(vec.get(idx).is_none(), idx >= size);
        }
    }
}

#[kernel_test]
fn test_vector_no_leak() {
    struct BigStruct {
        pub val: [u64; 10000],
    }

    let mut rng = Rng::new(234567890987654);
    for _ in 0..10000 {
        let _ = unsafe { Vec::<BigStruct>::new_with_size_uninit(rng.get(0, 100) as usize) };
    }
}

fn test_vector_calls_drop_on_delete() {
    struct BigStruct {
        pub val: [u64; 10000],
    }

    let mut rng = Rng::new(234567890987654);
    for _ in 0..10000 {
        let mut vec = Vec::new();
        let len = rng.get(0, 100);
        for _ in 0..len {
            vec.push(unsafe { Box::<BigStruct>::new_uninit() });
        }
    }
}

fn test_vector_calls_drop_on_pop() {
    struct BigStruct {
        pub val: [u64; 10000],
    }

    let mut rng = Rng::new(678543456378);
    let mut vec = Vec::new();
    for _ in 0..10000 {
        let len = rng.get(0, 100);
        for _ in 0..len {
            vec.push(unsafe { Box::<BigStruct>::new_uninit() });
        }
        for _ in 0..len {
            vec.pop();
        }
    }
}
