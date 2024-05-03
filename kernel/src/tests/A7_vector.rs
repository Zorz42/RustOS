use kernel_test::{kernel_test, kernel_test_mod};
use std::{Rng, Vec};
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
