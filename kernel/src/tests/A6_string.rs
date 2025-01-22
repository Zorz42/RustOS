use kernel_test::{kernel_test, kernel_test_mod};
use kernel_std::{String, Vec};
kernel_test_mod!(crate::tests::A6_string);

#[kernel_test]
fn test_string_split() {
    let str1 = String::from("/home/jakob/directory/file");
    let parts = str1.split('/');

    assert!(parts == Vec::new_from_slice(&[String::from(""), String::from("home"), String::from("jakob"), String::from("directory"), String::from("file")]))
}
