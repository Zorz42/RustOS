// always operates with the currently mounted disk

use std::Vec;

pub fn fs_erase() {
    todo!();
}

pub fn fs_read_file(path: &str) -> Vec<u8> {
    todo!();
}

pub fn fs_write_file(path: &str, data: &Vec<u8>) {
    todo!();
}

pub fn fs_read_program(name: &str) -> Vec<u8> {
    todo!();
}

pub fn fs_write_program(name: &str, data: &Vec<u8>) {
    todo!();
}

pub fn fs_get_program_mapped_pages(name: &str) -> Vec<(i32, i32)> {
    todo!();
}

pub fn fs_map_new_page_for_program(name: &str, page_addr: *mut u8) {
    todo!();
}