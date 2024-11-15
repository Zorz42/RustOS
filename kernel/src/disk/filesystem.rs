use kernel_std::{deserialize, println, serialize, String, Vec};
use kernel_std::derive::Serial;
use crate::disk::memory_disk::get_mounted_disk;
use crate::memory::PAGE_SIZE;

fn read_pages_from_disk(pages: &Vec<usize>, size: usize) -> Vec<u8> {
    let t = get_mounted_disk().borrow();
    let min_size = pages.size() * PAGE_SIZE as usize - PAGE_SIZE as usize + 1;
    let max_size = pages.size() * PAGE_SIZE as usize;
    assert!(size >= min_size && size <= max_size);
    let mut res = Vec::new();

    let mut size_left = size;
    for page in pages {
        let page_data = get_mounted_disk().get_mut(&t).as_mut().unwrap().read_page(*page as i32);
        for i in 0..PAGE_SIZE {
            if size_left == 0 {
                break;
            }
            size_left -= 1;
            res.push(page_data[i as usize]);
        }
    }

    get_mounted_disk().release(t);

    res
}

fn write_to_pages_on_disk(pages: &Vec<usize>, data: &Vec<u8>) {
    let t = get_mounted_disk().borrow();
    let min_size = pages.size() * PAGE_SIZE as usize - PAGE_SIZE as usize + 1;
    let max_size = pages.size() * PAGE_SIZE as usize;
    assert!(data.size() >= min_size && data.size() <= max_size);

    let mut size_left = data.size();
    for page in pages {
        let mut page_data = [0; PAGE_SIZE as usize];
        for i in 0..PAGE_SIZE {
            if size_left == 0 {
                break;
            }
            page_data[i as usize] = data[data.size() - size_left];
            size_left -= 1;
        }
        get_mounted_disk().get_mut(&t).as_mut().unwrap().write_page(*page as i32, &page_data);
    }

    get_mounted_disk().release(t);
}

#[derive(Serial)]
struct Directory {
    subdirectories: Vec::<(Vec::<usize>,usize,String)>, // each directory is a tuple of (pages, size, name)
}

fn load_directory(pages: &Vec<usize>, size: usize) -> Directory {
    let data = read_pages_from_disk(pages, size);
    let res = deserialize(&data);

    res
}

fn delete_pages(pages: &Vec<usize>) {
    let t = get_mounted_disk().borrow();
    for page in pages {
        get_mounted_disk().get_mut(&t).as_mut().unwrap().free_page(*page as i32);
    }
    get_mounted_disk().release(t);
}

fn store_directory(directory: &mut Directory) -> (Vec<usize>, usize) {
    let data = serialize(directory);
    let mut pages = Vec::new();
    let size = data.size();
    let t = get_mounted_disk().borrow();
    let num_pages = (size + PAGE_SIZE as usize - 1) / PAGE_SIZE as usize;
    for _ in 0..num_pages {
        pages.push(get_mounted_disk().get_mut(&t).as_mut().unwrap().alloc_page() as usize);
    }
    get_mounted_disk().release(t);
    write_to_pages_on_disk(&pages, &data);
    (pages, data.size())
}

pub fn fs_erase() {
    let t = get_mounted_disk().borrow();
    get_mounted_disk().get_mut(&t).as_mut().unwrap().erase();
    get_mounted_disk().release(t);

    let mut root = Directory {
        subdirectories: Vec::new(),
    };

    set_root(&mut root);
}

fn get_root() -> Directory {
    let t = get_mounted_disk().borrow();
    let head = get_mounted_disk().get_mut(&t).as_mut().unwrap().get_head();
    get_mounted_disk().release(t);

    let (pages, size) = deserialize(&head);
    load_directory(&pages, size)
}

fn set_root(root: &mut Directory) {
    let (pages, size) = store_directory(root);
    let head = serialize(&mut (pages, size));

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

pub fn create_directory(path: &String) {
    let mut dirs = Vec::new();
    dirs.push(get_root());
    let path = parse_path(path);

    for dir in &path {
        let dir_entry = {
            let curr_dir = &dirs[dirs.size() - 1];
            let mut dir_entry = None;
            for entry in &curr_dir.subdirectories {
                if &entry.2 == dir {
                    dir_entry = Some(entry);
                }
            }
            dir_entry.cloned()
        };

        // remove the directory as it will be reinserted
        let idx = dirs.size() - 1;
        dirs[idx].subdirectories.retain(&|entry| &entry.2 != dir);

        if let Some((pages, size, _)) = dir_entry {
            dirs.push(load_directory(&pages, size));
            delete_pages(&pages);
        } else {
            let new_dir = Directory {
                subdirectories: Vec::new(),
            };
            dirs.push(new_dir);
        }
    }

    // store all directories to disk
    for i in (1..dirs.size()).rev() {
        let (pages, size) = store_directory(&mut dirs[i]);
        let name = path[i - 1].clone();
        dirs[i - 1].subdirectories.push((pages, size, name));
    }

    set_root(&mut dirs[0]);
}

pub fn is_directory(path: &String) -> bool {
    let mut dirs = Vec::new();
    dirs.push(get_root());
    let path = parse_path(path);

    for dir in &path {
        let curr_dir = &dirs[dirs.size() - 1];
        let mut dir_entry = None;
        for entry in &curr_dir.subdirectories {
            if &entry.2 == dir {
                dir_entry = Some(entry);
            }
        }

        if let Some((pages, size, _)) = dir_entry {
            dirs.push(load_directory(&pages, *size));
        } else {
            return false;
        }
    }

    true
}

pub fn delete_directory(path: &String) {

}