// always operates with the currently mounted disk

use std::{Serial, String, Vec};
use crate::memory_disk::{DiskBox, get_mounted_disk};

#[derive(std::derive::Serial)]
struct File {
    name: String,
    page: i32,
}

#[derive(std::derive::Serial)]
struct Directory {
    name: String,
    files: Vec::<File>,
    subdirs: Vec::<DiskBox<Directory>>,
}

impl Directory {
    pub fn new(name: String) -> Self {
        Self {
            name,
            files: Vec::new(),
            subdirs: Vec::new(),
        }
    }
}

pub struct FileSystem {
    root: DiskBox<Directory>,
}

impl FileSystem {
    pub fn new() -> Self {
        Self {
            root: DiskBox::new_at_page(get_mounted_disk().get_master_page()),
        }
    }

    pub fn erase(&mut self) {
        get_mounted_disk().erase();
        *self.root.get() = Directory::new(String::new());
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

static mut FILESYSTEM: Option<FileSystem> = None;

pub fn init_fs() {
    unsafe {
        FILESYSTEM = Some(FileSystem::new());
    }
}

pub fn get_fs() -> &'static mut FileSystem {
    unsafe {
        FILESYSTEM.as_mut().unwrap()
    }
}
