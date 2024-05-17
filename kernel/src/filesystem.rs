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

    // path is reversed, so you can pop it when going to the next folder
    fn get_directory_full(&mut self, mut path: Vec<String>) -> Option<&mut Directory> {
        if let Some(dir) = path.pop() {
            if let Some(subdir) = self.get_directory(&dir) {
                subdir.get_directory_full(path)
            } else {
                None
            }
        } else {
            Some(self)
        }
    }

    pub fn get_directory(&mut self, name: &String) -> Option<&mut Directory> {
        for mut subdir in &mut self.subdirs {
            if subdir.get().name == *name {
                return Some(subdir.get());
            }
        }
        None
    }

    pub fn get_file(&mut self, name: &String) -> Option<&mut File> {
        for mut file in &mut self.files {
            if file.name == *name {
                return Some(file);
            }
        }
        None
    }

    pub fn create_directory(&mut self, name: String) -> &mut Directory {
        if self.get_directory(&name).is_some() {
            self.get_directory(&name).unwrap()
        } else {
            self.subdirs.push(DiskBox::new(Directory::new(name))).get()
        }
    }

    pub fn create_file(&mut self, name: String) -> &mut File {
        if self.get_file(&name).is_some() {
            self.get_file(&name).unwrap()
        } else {
            self.files.push(File::new(name))
        }
    }

    fn create_directory_full(&mut self, mut dirs: Vec<String>) -> &mut Directory {
        if let Some(dir_name) = dirs.pop() {
            let dir = self.create_directory(dir_name);
            dir.create_directory_full(dirs)
        } else {
            self
        }
    }
    
    pub fn delete_file(&mut self, name: &String) {
        self.files.retain(&|file| file.name != *name);
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
        self.root = DiskBox::new(Directory::new(String::new()));
    }

    pub fn get_directory(&mut self, path: &String) -> Option<&mut Directory> {
        let mut parts = path.split('/');
        parts.retain(&|x| x.size() != 0);
        parts.reverse();
        self.root.get().get_directory_full(parts)
    }

    pub fn get_file(&mut self, path: &String) -> Option<&mut File> {
        let mut parts = path.split('/');
        parts.retain(&|x| x.size() != 0);
        if let Some(file_name) = parts.pop() {
            parts.reverse();
            if let Some(directory) = self.root.get().get_directory_full(parts) {
                directory.get_file(&file_name)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn create_file(&mut self, path: &String) -> &mut File {
        let mut parts = path.split('/');
        parts.retain(&|x| x.size() != 0);
        if let Some(file_name) = parts.pop() {
            parts.reverse();
            let parent = self.root.get().create_directory_full(parts);
            parent.create_file(file_name)
        } else {
            panic!("No file name specified!");
        }
    }

    pub fn create_directory(&mut self, path: &String) -> &mut Directory {
        todo!();
    }

    pub fn delete_file(&mut self, path: &String) {
        let mut parts = path.split('/');
        parts.retain(&|x| x.size() != 0);
        if let Some(file_name) = parts.pop() {
            parts.reverse();
            let parent = self.root.get().get_directory_full(parts).unwrap();
            parent.delete_file(&file_name);
        } else {
            panic!("No file name specified!");
        }
    }

    pub fn delete_directory(&mut self, path: &String) {
        todo!();
    }
}

impl Drop for FileSystem {
    fn drop(&mut self) {
        get_mounted_disk().set_head(&serialize(&mut self.root));
    }
}

static mut FILESYSTEM: Option<FileSystem> = None;

pub fn init_fs() {
    unsafe {
        FILESYSTEM = Some(FileSystem::new());
    }
}

pub fn close_fs() {
    unsafe {
        FILESYSTEM = None;
    }
}

pub fn get_fs() -> &'static mut FileSystem {
    unsafe {
        FILESYSTEM.as_mut().unwrap()
    }
}
