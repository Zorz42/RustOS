use kernel_test::{kernel_test, kernel_test_mod};

kernel_test_mod!(crate::tests::A8_disk);

#[kernel_test]
fn test_disk_write() {}
