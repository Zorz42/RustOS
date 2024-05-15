// always operates with the currently mounted disk

use std::{deserialize, Serial, serialize, String, Vec};
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
        if get_mounted_disk().get_head().size() == 0 {
            get_mounted_disk().set_head(&serialize(&mut DiskBox::new(Directory::new(String::new()))));
        }
        
        Self {
            root: deserialize(&get_mounted_disk().get_head()),
        }
    }

    pub fn erase(&mut self) {
        get_mounted_disk().erase();
        self.root.set(Directory::new(String::new()));
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
