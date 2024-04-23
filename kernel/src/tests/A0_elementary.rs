use kernel_test::{kernel_test, kernel_test_mod};

kernel_test_mod!(crate::tests::A0_elementary);

#[kernel_test]
fn test_assert() {
    assert!(true);
}
