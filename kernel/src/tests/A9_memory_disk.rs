use kernel_test::{kernel_test, kernel_test_mod};
use crate::memory_disk::mount_disk;
use crate::tests::get_test_disk;
kernel_test_mod!(crate::tests::A9_memory_disk);

#[kernel_test]
fn test_disk_create_destroy() {

}

#[kernel_test]
fn test_disk_save() {

}

#[kernel_test]
fn test_disk_load() {

}

#[kernel_test]
fn test_disk_save_load() {

}