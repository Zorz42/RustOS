// always operates with the currently mounted disk

use std::Vec;

pub struct FileSystem {

}

impl FileSystem {
    pub const fn new() -> Self {
        Self {

        }
    }

    pub fn init(&self) {
        todo!();
    }

    pub fn erase(&self) {
        todo!();
    }

    pub fn read_file(&self, path: &str) -> Vec<u8> {
        todo!();
    }

    pub fn write_file(&self, path: &str, data: &Vec<u8>) {
        todo!();
    }

    pub fn program_name_to_id(&self, name: &str) -> i32 {
        todo!();
    }

    pub fn read_program(&self, id: i32) -> Vec<u8> {
        todo!();
    }

    pub fn write_program(&self, id: i32, data: &Vec<u8>) {
        todo!();
    }

    pub fn get_program_mapped_pages(&self, id: i32) -> Vec<(i32, i32)> {
        todo!();
    }

    pub fn map_new_page_for_program(&self, id: i32, page_addr: *mut u8) {
        todo!();
    }
}

static mut FILESYSTEM: FileSystem = FileSystem::new();

pub fn get_fs() -> &'static mut FileSystem {
    unsafe {
        &mut FILESYSTEM
    }
}
