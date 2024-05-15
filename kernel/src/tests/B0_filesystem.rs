use kernel_test::{kernel_test, kernel_test_mod};
use crate::filesystem::get_fs;

kernel_test_mod!(crate::tests::B0_filesystem);

#[kernel_test]
fn test_erase_fs() {
    for _ in 0..100 {
        get_fs().erase();
    }
}