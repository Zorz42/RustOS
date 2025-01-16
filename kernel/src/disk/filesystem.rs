use kernel_std::{deserialize, serialize, String, Vec};
use kernel_std::derive::Serial;
use crate::disk::disk::SECTOR_SIZE;
use crate::disk::memory_disk::get_mounted_disk;

fn read_sectors_from_disk(sectors: &Vec<usize>, size: usize) -> Vec<u8> {
    let t = get_mounted_disk().borrow();
    let min_size = sectors.size() * SECTOR_SIZE - SECTOR_SIZE + 1;
    let max_size = sectors.size() * SECTOR_SIZE;
    assert!(size >= min_size && size <= max_size);
    let mut res = Vec::new();

    let mut size_left = size;
    for sector in sectors {
        let sector_data = get_mounted_disk().get_mut(&t).as_mut().unwrap().read_sector(*sector);
        for i in sector_data {
            if size_left == 0 {
                break;
            }
            size_left -= 1;
            res.push(i);
        }
    }

    get_mounted_disk().release(t);

    res
}

fn write_to_sectors_on_disk(sectors: &Vec<usize>, data: &Vec<u8>) {
    let t = get_mounted_disk().borrow();
    let min_size = sectors.size() as i32 * SECTOR_SIZE as i32 - SECTOR_SIZE as i32 + 1;
    let max_size = sectors.size() as i32 * SECTOR_SIZE as i32;
    assert!(data.size() as i32 >= min_size && data.size() as i32 <= max_size);

    let mut size_left = data.size();
    for sector in sectors {
        let mut sector_data = [0; SECTOR_SIZE];
        for i in &mut sector_data {
            if size_left == 0 {
                break;
            }
            *i = data[data.size() - size_left];
            size_left -= 1;
        }
        get_mounted_disk().get_mut(&t).as_mut().unwrap().write_sector(*sector, &sector_data);
    }

    get_mounted_disk().release(t);
}

#[derive(Serial)]
struct Directory {
    subdirectories: Vec::<(Vec::<usize>,usize,String)>, // each directory is a tuple of (sectors, size, name)
    files: Vec::<(Vec::<usize>,usize,String)>, // each file is a tuple of (sectors, size, name)
}

impl Directory {
    fn new() -> Self {
        Self {
            subdirectories: Vec::new(),
            files: Vec::new(),
        }
    }

    fn get_subdirectory(&self, name: &String) -> Option<&(Vec<usize>, usize, String)> {
        (&self.subdirectories).into_iter().find(|&entry| &entry.2 == name)
    }

    fn get_file(&self, name: &String) -> Option<&(Vec<usize>, usize, String)> {
        (&self.files).into_iter().find(|&entry| &entry.2 == name)
    }
}

fn load_directory(sectors: &Vec<usize>, size: usize) -> Directory {
    let data = read_sectors_from_disk(sectors, size);
    deserialize(&data)
}

fn delete_sectors(sectors: &Vec<usize>) {
    let t = get_mounted_disk().borrow();
    for sector in sectors {
        get_mounted_disk().get_mut(&t).as_mut().unwrap().free_sector(*sector);
    }
    get_mounted_disk().release(t);
}

fn store_directory(directory: &mut Directory) -> (Vec<usize>, usize) {
    let data = serialize(directory);
    let mut sectors = Vec::new();
    let size = data.size();
    let t = get_mounted_disk().borrow();
    let num_sectors = size.div_ceil(SECTOR_SIZE);
    for _ in 0..num_sectors {
        sectors.push(get_mounted_disk().get_mut(&t).as_mut().unwrap().alloc_sector());
    }
    get_mounted_disk().release(t);
    write_to_sectors_on_disk(&sectors, &data);
    (sectors, data.size())
}

pub fn fs_erase() {
    let t = get_mounted_disk().borrow();
    get_mounted_disk().get_mut(&t).as_mut().unwrap().erase();
    get_mounted_disk().release(t);

    let mut root = Directory::new();

    set_root(&mut root);
}

fn get_root() -> Directory {
    let t = get_mounted_disk().borrow();
    let head = get_mounted_disk().get_mut(&t).as_mut().unwrap().get_head();
    get_mounted_disk().release(t);

    let (sectors, size) = deserialize(&head);
    load_directory(&sectors, size)
}

fn set_root(root: &mut Directory) {
    let t = get_mounted_disk().borrow();
    let head = get_mounted_disk().get_mut(&t).as_mut().unwrap().get_head();
    get_mounted_disk().release(t);

    if head.size() != 0 {
        let (sectors, _) = deserialize::<(Vec<usize>, usize)>(&head);
        delete_sectors(&sectors);
    }

    let (sectors, size) = store_directory(root);
    let head = serialize(&mut (sectors, size));

    let t = get_mounted_disk().borrow();
    get_mounted_disk().get_mut(&t).as_mut().unwrap().set_head(&head);
    get_mounted_disk().release(t);
}

fn parse_path(path: &String) -> Vec<String> {
    let mut res = Vec::new();
    let mut curr = String::new();
    for c in path {
        if *c == '/' {
            if curr.size() > 0 {
                res.push(curr);
                curr = String::new();
            }
        } else {
            curr.push(*c);
        }
    }
    if curr.size() > 0 {
        res.push(curr);
    }

    let mut res2 = Vec::new();
    for i in res {
        if i == String::from(".") {
            continue;
        }

        if i == String::from("..") {
            res2.pop();
            continue;
        }

        res2.push(i);
    }

    res2
}

fn store_directory_chain(dirs: &mut Vec<Directory>, path: &Vec<String>) {
    for i in (1..dirs.size()).rev() {
        let (sectors, size) = store_directory(&mut dirs[i]);
        let name = path[i - 1].clone();
        dirs[i - 1].subdirectories.push((sectors, size, name));
    }

    set_root(&mut dirs[0]);
}

pub fn create_directory(path: &String) {
    let mut dirs = Vec::new();
    dirs.push(get_root());
    let path = parse_path(path);

    for dir in &path {
        let dir_entry = dirs[dirs.size() - 1].get_subdirectory(dir).cloned();

        // remove the directory as it will be reinserted
        let idx = dirs.size() - 1;
        dirs[idx].subdirectories.retain(&|entry| &entry.2 != dir);

        if let Some((sectors, size, _)) = dir_entry {
            dirs.push(load_directory(&sectors, size));
            delete_sectors(&sectors);
        } else {
            let new_dir = Directory::new();
            dirs.push(new_dir);
        }
    }

    store_directory_chain(&mut dirs, &path);
}

pub fn is_directory(path: &String) -> bool {
    let mut dirs = Vec::new();
    dirs.push(get_root());
    let path = parse_path(path);

    for dir in &path {
        let dir_entry = dirs[dirs.size() - 1].get_subdirectory(dir).cloned();

        if let Some((sectors, size, _)) = dir_entry {
            dirs.push(load_directory(&sectors, size));
        } else {
            return false;
        }
    }

    true
}

pub fn delete_directory(path: &String) {
    let mut dirs = Vec::new();
    dirs.push(get_root());
    let path = parse_path(path);

    for dir in &path {
        let dir_entry = dirs[dirs.size() - 1].get_subdirectory(dir).cloned();

        if let Some((sectors, size, _)) = dir_entry {
            dirs.push(load_directory(&sectors, size));
            delete_sectors(&sectors);
        } else {
            return;
        }
    }

    // remove last dir
    dirs.pop();

    // also remove its reference from the parent
    let idx = dirs.size() - 1;
    dirs[idx].subdirectories.retain(&|entry| entry.2 != path[path.size() - 1]);

    store_directory_chain(&mut dirs, &path);
}

pub fn write_to_file(path: &String, data: &Vec<u8>) {
    delete_file(path);

    let mut dirs = Vec::new();
    dirs.push(get_root());
    let mut path = parse_path(path);
    let file_name = path.pop().unwrap();

    for dir in &path {
        let dir_entry = dirs[dirs.size() - 1].get_subdirectory(dir).cloned();

        // remove the directory as it will be reinserted
        let idx = dirs.size() - 1;
        dirs[idx].subdirectories.retain(&|entry| &entry.2 != dir);

        if let Some((sectors, size, _)) = dir_entry {
            dirs.push(load_directory(&sectors, size));
            delete_sectors(&sectors);
        } else {
            let new_dir = Directory::new();
            dirs.push(new_dir);
        }
    }

    let idx = dirs.size() - 1;
    let parent_dir = &mut dirs[idx];

    let mut sectors = Vec::new();
    let sectors_count = data.size().div_ceil(SECTOR_SIZE);
    let t = get_mounted_disk().borrow();
    for _ in 0..sectors_count {
        sectors.push(get_mounted_disk().get_mut(&t).as_mut().unwrap().alloc_sector());
    }
    get_mounted_disk().release(t);
    write_to_sectors_on_disk(&sectors, data);

    parent_dir.files.push((sectors, data.size(), file_name));

    store_directory_chain(&mut dirs, &path);
}

pub fn is_file(path: &String) -> bool {
    let mut dirs = Vec::new();
    dirs.push(get_root());
    let mut path = parse_path(path);
    let file_name = path.pop().unwrap();

    for dir in &path {
        let dir_entry = dirs[dirs.size() - 1].get_subdirectory(dir).cloned();

        if let Some((sectors, size, _)) = dir_entry {
            dirs.push(load_directory(&sectors, size));
        } else {
            return false;
        }
    }

    let idx = dirs.size() - 1;
    let parent_dir = &mut dirs[idx];
    parent_dir.get_file(&file_name).is_some()
}

pub fn delete_file(path: &String) {
    let mut dirs = Vec::new();
    dirs.push(get_root());
    let mut path = parse_path(path);
    let file_name = path.pop().unwrap();

    for dir in &path {
        let dir_entry = dirs[dirs.size() - 1].get_subdirectory(dir).cloned();

        // remove the directory as it will be reinserted
        let idx = dirs.size() - 1;
        dirs[idx].subdirectories.retain(&|entry| &entry.2 != dir);

        if let Some((sectors, size, _)) = dir_entry {
            dirs.push(load_directory(&sectors, size));
            delete_sectors(&sectors);
        } else {
            let new_dir = Directory::new();
            dirs.push(new_dir);
        }
    }

    let idx = dirs.size() - 1;
    let parent_dir = &mut dirs[idx];
    for (sectors, _, name) in &parent_dir.files {
        if *name == file_name {
            delete_sectors(sectors);
        }
    }

    parent_dir.files.retain(&|entry| entry.2 != file_name);

    store_directory_chain(&mut dirs, &path);
}

pub fn read_file(path: &String) -> Option<Vec<u8>> {
    let mut dirs = Vec::new();
    dirs.push(get_root());
    let mut path = parse_path(path);
    let file_name = path.pop().unwrap();

    for dir in &path {
        let dir_entry = dirs[dirs.size() - 1].get_subdirectory(dir).cloned();

        if let Some((sectors, size, _)) = dir_entry {
            dirs.push(load_directory(&sectors, size));
        } else {
            return None;
        }
    }

    let idx = dirs.size() - 1;
    let parent_dir = &mut dirs[idx];
    let file_entry = parent_dir.get_file(&file_name).cloned();

    if let Some((sectors, size, _)) = file_entry {
        Some(read_sectors_from_disk(&sectors, size))
    } else {
        None
    }
}

// returns (dirs, files)
pub fn list_directory(path: &String) -> Option<(Vec<String>, Vec<String>)> {
    let mut dirs = Vec::new();
    dirs.push(get_root());
    let path = parse_path(path);

    for dir in &path {
        let dir_entry = dirs[dirs.size() - 1].get_subdirectory(dir).cloned();

        if let Some((sectors, size, _)) = dir_entry {
            dirs.push(load_directory(&sectors, size));
        } else {
            return None;
        }
    }

    let idx = dirs.size() - 1;
    let dir = &mut dirs[idx];
    let mut files = Vec::new();
    for (_, _, name) in &dir.files {
        files.push(name.clone());
    }
    let mut dirs = Vec::new();
    for (_, _, name) in &dir.subdirectories {
        dirs.push(name.clone());
    }
    Some((dirs, files))
}