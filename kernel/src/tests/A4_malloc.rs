use kernel_test::{kernel_test, kernel_test_mod};

use crate::memory::{free, malloc};
use crate::rand::Rng;

kernel_test_mod!(crate::tests::A4_malloc);

#[kernel_test]
fn test_malloc() {
    let mut rng = Rng::new(754389);
    let _ = malloc(0);
    for _ in 0..100 {
        let _ = malloc(rng.get(0, 100) as usize);
    }
}

#[kernel_test]
fn test_malloc_free() {
    let mut rng = Rng::new(6437892);
    let _ = malloc(0);
    for _ in 0..100 {
        let ptr = malloc(rng.get(0, 10000) as usize);
        unsafe {
            free(ptr);
        }
    }
}
