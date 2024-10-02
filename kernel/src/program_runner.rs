use std::{println, String};
use crate::disk::filesystem::get_fs;

pub fn run_program(path: &String) {
    println!("Running program: {}", path);

    let program = get_fs().get_file(path).unwrap().read();

}