// always operates with the currently mounted disk

use std::{deserialize, Serial, serialize, String, Vec};
use crate::memory_disk::{DiskBox, get_mounted_disk};

#[derive(std::derive::Serial)]
pub struct File {
    name: String,
    pages: Vec::<i32>,
}

impl File {
    fn new(name: String) -> Self {
        Self {
            name,
            pages: Vec::new(),
        }
    }
}

#[derive(std::derive::Serial)]
pub struct Directory {
    name: String,
    files: Vec::<File>,
    subdirs: Vec::<DiskBox<Directory>>,
}

impl Directory {
    fn new(name: String) -> Self {
        Self {
            name,
            files: Vec::new(),
            subdirs: Vec::new(),
        }
    }

    fn get_directory(&mut self, mut path: Vec<String>) -> Option<&mut Directory> {
        if let Some(dir) = path.pop() {
            for mut subdir in &mut self.subdirs {
                if subdir.get().name == dir {
                    return subdir.get().get_directory(path)
                }
            }
            None
        } else {
            Some(self)
        }
    }
}

pub struct FileSystem {
    root: DiskBox<Directory>,
}

fn parse_path(path: &String) -> Vec<String> {
    let parts = path.split('/');
    let mut res = Vec::new();
    for i in parts {
        if i.size() != 0 {
            res.push(i);
        }
    }
    res
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

    pub fn get_directory(&mut self, path: &String) -> Option<&mut Directory> {
        todo!();
    }

    pub fn get_file(&mut self, path: &String) -> Option<&mut File> {
        todo!();
    }

    pub fn create_file(&mut self, path: &String) -> &mut File {
        todo!();
    }

    pub fn delete_file(&mut self, path: &String) {
        todo!();
    }

    pub fn create_directory(&mut self, path: &String) -> &mut File {
        todo!();
    }

    pub fn delete_directory(&mut self, path: &String) {
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
