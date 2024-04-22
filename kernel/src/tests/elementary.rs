use kernel_test::kernel_test;

#[kernel_test(crate::tests::elementary)]
fn test_assert() {
    assert!(true);
}
