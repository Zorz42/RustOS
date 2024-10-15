// always operates with the currently mounted disk

use core::mem::swap;
use crate::memory::{DISK_OFFSET, PAGE_SIZE};
use crate::disk::memory_disk::{get_mounted_disk, DiskBox};
use kernel_std::{deserialize, serialize, String, Vec, Box, Mutable};

pub struct Path {
    dirs: Vec<String>,
}

impl Path {
    pub fn from(string: &String) -> Self {
        let mut dirs = string.split('/');
        dirs.retain(&|x| x.size() != 0 && *x != String::from("."));
        let mut dirs2 = Vec::new();
        for dir in dirs {
            if dir == String::from("..") {
                dirs2.pop();
            } else {
                dirs2.push(dir);
            }
        }
        Self {
            dirs: dirs2,
        }
    }

    pub fn to_string(&self) -> String {
        let mut res = String::new();

        for i in &self.dirs {
            res.push('/');
            for c in i {
                res.push(*c);
            }
        }
        res.push('/');

        res
    }
}

#[derive(kernel_std::derive::Serial)]
pub struct File {
    name: String,
    size: i32,
    pages: Vec::<i32>,
}

impl File {
    fn new(name: String) -> Self {
        Self { name, size: 0, pages: Vec::new() }
    }

    pub const fn get_name(&self) -> &String {
        &self.name
    }

    pub fn read(&self) -> Vec<u8> {
        let t = get_mounted_disk().borrow();
        let mut res = Vec::new();

        for i in 0..self.size {
            let page = self.pages[(i / (PAGE_SIZE as i32)) as usize];
            if i as u64 % PAGE_SIZE == 0 {
                get_mounted_disk().get_mut(&t).as_mut().unwrap().declare_read(DISK_OFFSET + page as u64 * PAGE_SIZE, DISK_OFFSET + page as u64 * PAGE_SIZE + PAGE_SIZE);
            }
            let addr = (DISK_OFFSET + page as u64 * PAGE_SIZE + (i as u64) % PAGE_SIZE) as *const u8;
            unsafe {
                res.push(*addr);
            }
        }
        get_mounted_disk().release(t);
        res
    }

    pub fn write(&mut self, data: &Vec<u8>) {
        self.clear();
        let t = get_mounted_disk().borrow();
        self.size = data.size() as i32;
        let num_pages = (self.size as u64).div_ceil(PAGE_SIZE);
        for _ in 0..num_pages {
            self.pages.push(get_mounted_disk().get_mut(&t).as_mut().unwrap().alloc_page());
        }
        for i in 0..self.size {
            let page = self.pages[(i / (PAGE_SIZE as i32)) as usize];
            if i as u64 % PAGE_SIZE == 0 {
                get_mounted_disk().get_mut(&t).as_mut().unwrap().declare_write(DISK_OFFSET + page as u64 * PAGE_SIZE, DISK_OFFSET + page as u64 * PAGE_SIZE + PAGE_SIZE);
            }
            let addr = (DISK_OFFSET + page as u64 * PAGE_SIZE + (i as u64) % PAGE_SIZE) as *mut u8;
            unsafe {
                *addr = data[i as usize];
            }
        }
        get_mounted_disk().release(t);
    }

    fn clear(&mut self) {
        let t = get_mounted_disk().borrow();
        self.size = 0;
        for page in &self.pages {
            get_mounted_disk().get_mut(&t).as_mut().unwrap().free_page(*page);
        }
        self.pages = Vec::new();
        get_mounted_disk().release(t);
    }
}

#[derive(kernel_std::derive::Serial)]
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
    fn get_directory_full(&mut self, mut path: Vec<String>) -> Option<&mut Self> {
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

    pub fn get_directory(&mut self, name: &String) -> Option<&mut Self> {
        for subdir in &mut self.subdirs {
            if subdir.get().name == *name {
                return Some(subdir.get());
            }
        }
        None
    }

    pub fn get_file(&mut self, name: &String) -> Option<&mut File> {
        (&mut self.files).into_iter().find(|file| file.name == *name)
    }

    pub fn create_directory(&mut self, name: String) -> &mut Self {
        if self.get_directory(&name).is_some() {
            self.get_directory(&name).unwrap()
        } else {
            self.subdirs.push(DiskBox::new(Self::new(name))).get()
        }
    }

    pub fn create_file(&mut self, name: String) -> &mut File {
        if self.get_file(&name).is_some() {
            self.get_file(&name).unwrap()
        } else {
            self.files.push(File::new(name))
        }
    }

    fn create_directory_full(&mut self, mut dirs: Vec<String>) -> &mut Self {
        if let Some(dir_name) = dirs.pop() {
            let dir = self.create_directory(dir_name);
            dir.create_directory_full(dirs)
        } else {
            self
        }
    }

    pub fn delete_file(&mut self, name: &String) {
        for file in &mut self.files {
            if file.name == *name {
                file.clear();
            }
        }
        self.files.retain(&|file| file.name != *name);
    }

    pub fn clear(&mut self) {
        for file in &mut self.files {
            file.clear();
        }
        self.files = Vec::new();
        let mut dirs = Vec::new();
        swap(&mut self.subdirs, &mut dirs);
        for mut dir in dirs {
            dir.get().clear();
            DiskBox::delete(dir);
        }
    }

    pub fn delete_directory(&mut self, name: &String) {
        let mut old_dirs = Vec::new();
        swap(&mut self.subdirs, &mut old_dirs);

        for mut dir in old_dirs {
            if dir.get().name == *name {
                dir.get().clear();
                DiskBox::delete(dir);
            } else {
                self.subdirs.push(dir);
            }
        }
    }

    pub const fn get_files(&self) -> &Vec<File> {
        &self.files
    }

    pub fn get_directories(&mut self) -> &mut Vec<DiskBox<Self>> {
        &mut self.subdirs
    }

    pub const fn get_name(&self) -> &String {
        &self.name
    }
}

pub struct FileSystem {
    root: Option<DiskBox<Directory>>,
}

impl FileSystem {
    fn new() -> Self {
        let t = get_mounted_disk().borrow();
        let size = get_mounted_disk().get_mut(&t).as_mut().unwrap().get_head().size();
        get_mounted_disk().release(t);
        if size == 0 {
            let vec = serialize(&mut DiskBox::new(Directory::new(String::new())));
            let t = get_mounted_disk().borrow();
            get_mounted_disk().get_mut(&t).as_mut().unwrap().set_head(&vec);
            get_mounted_disk().release(t);
        }

        let t = get_mounted_disk().borrow();
        let result = Self {
            root: Some(deserialize(&get_mounted_disk().get_mut(&t).as_mut().unwrap().get_head())),
        };
        get_mounted_disk().release(t);
        result
    }

    pub fn erase(&mut self) {
        self.root = None;
        let t = get_mounted_disk().borrow();
        get_mounted_disk().get_mut(&t).as_mut().unwrap().erase();
        get_mounted_disk().release(t);
        self.root = Some(DiskBox::new(Directory::new(String::new())));
    }

    pub fn get_directory(&mut self, path: &String) -> Option<&mut Directory> {
        let path = Path::from(path);
        let mut parts = path.dirs;
        parts.reverse();
        self.root.as_mut().unwrap().get().get_directory_full(parts)
    }

    pub fn get_file(&mut self, path: &String) -> Option<&mut File> {
        let path = Path::from(path);
        let mut parts = path.dirs;
        let file_name = parts.pop()?;
        parts.reverse();
        let directory = self.root.as_mut().unwrap().get().get_directory_full(parts)?;
        directory.get_file(&file_name)
    }

    pub fn create_file(&mut self, path: &String) -> &mut File {
        let path = Path::from(path);
        let mut parts = path.dirs;
        if let Some(file_name) = parts.pop() {
            parts.reverse();
            let parent = self.root.as_mut().unwrap().get().create_directory_full(parts);
            parent.create_file(file_name)
        } else {
            panic!("No file name specified!");
        }
    }

    pub fn create_directory(&mut self, path: &String) -> &mut Directory {
        let path = Path::from(path);
        let mut parts = path.dirs;
        parts.reverse();
        self.root.as_mut().unwrap().get().create_directory_full(parts)
    }

    pub fn delete_file(&mut self, path: &String) {
        let path = Path::from(path);
        let mut parts = path.dirs;
        if let Some(file_name) = parts.pop() {
            parts.reverse();
            let parent = self.root.as_mut().unwrap().get().get_directory_full(parts).unwrap();
            parent.delete_file(&file_name);
        } else {
            panic!("No file name specified!");
        }
    }

    pub fn delete_directory(&mut self, path: &String) {
        let path = Path::from(path);
        let mut parts = path.dirs;
        if let Some(dir_name) = parts.pop() {
            parts.reverse();
            let parent = self.root.as_mut().unwrap().get().get_directory_full(parts).unwrap();
            parent.delete_directory(&dir_name);
        } else {
            panic!("No directory name specified!");
        }
    }
}

impl Drop for FileSystem {
    fn drop(&mut self) {
        let vec = serialize(self.root.as_mut().unwrap());
        let t = get_mounted_disk().borrow();
        get_mounted_disk().get_mut(&t).as_mut().unwrap().set_head(&vec);
        get_mounted_disk().release(t);
    }
}

static FILESYSTEM: Mutable<Option<Box<FileSystem>>> = Mutable::new(None);

pub fn init_fs() {
    let t = FILESYSTEM.borrow();
    *FILESYSTEM.get_mut(&t) = Some(Box::new(FileSystem::new()));
    FILESYSTEM.release(t);
}

pub fn close_fs() {
    let t = FILESYSTEM.borrow();
    *FILESYSTEM.get_mut(&t) = None;
    FILESYSTEM.release(t);
}

pub fn get_fs() -> &'static mut FileSystem {
    let t = FILESYSTEM.borrow();
    let res = FILESYSTEM.get_mut(&t).as_mut().unwrap();
    FILESYSTEM.release(t);
    res
}