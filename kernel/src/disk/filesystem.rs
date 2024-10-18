use kernel_std::{deserialize, serialize, String, Vec};
use kernel_std::derive::Serial;
use crate::disk::memory_disk::get_mounted_disk;
use crate::memory::{DISK_OFFSET, PAGE_SIZE};

fn read_pages_from_disk(pages: &Vec<usize>, size: usize) -> Vec<u8> {
    let t = get_mounted_disk().borrow();
    let min_size = pages.size() * PAGE_SIZE as usize - PAGE_SIZE as usize + 1;
    let max_size = pages.size() * PAGE_SIZE as usize;
    assert!(size >= min_size && size <= max_size);
    let mut res = Vec::new();

    for page in pages {
        let page_addr = DISK_OFFSET + PAGE_SIZE * *page as u64;

        get_mounted_disk().get_mut(&t).as_mut().unwrap().declare_read(page_addr, page_addr + PAGE_SIZE);
    }

    for i in 0..size {
        let page_idx = pages[i / PAGE_SIZE as usize];
        let page_addr = DISK_OFFSET + PAGE_SIZE * page_idx as u64;
        let page_offset = i % PAGE_SIZE as usize;
        res.push(unsafe { (page_addr as *mut u8).add(page_offset).read() });
    }

    get_mounted_disk().release(t);

    res
}

fn write_to_pages_on_disk(pages: &Vec<usize>, data: &Vec<u8>) {
    let t = get_mounted_disk().borrow();
    let min_size = pages.size() * PAGE_SIZE as usize - PAGE_SIZE as usize + 1;
    let max_size = pages.size() * PAGE_SIZE as usize;
    assert!(data.size() >= min_size && data.size() <= max_size);

    for page in pages {
        let page_addr = DISK_OFFSET + PAGE_SIZE * *page as u64;

        get_mounted_disk().get_mut(&t).as_mut().unwrap().declare_write(page_addr, page_addr + PAGE_SIZE);
    }

    for i in 0..data.size() {
        let page_idx = pages[i / PAGE_SIZE as usize];
        let page_addr = DISK_OFFSET + PAGE_SIZE * page_idx as u64;
        let page_offset = i % PAGE_SIZE as usize;
        unsafe { (page_addr as *mut u8).add(page_offset).write(data[i]); }
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
    let t = get_mounted_disk().borrow();
    for page in pages {
        get_mounted_disk().get_mut(&t).as_mut().unwrap().free_page(*page as i32);
    }
    get_mounted_disk().release(t);
    res
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

    let (pages, size) = store_directory(&mut root);
    let head = serialize(&mut (pages, size));

    let t = get_mounted_disk().borrow();
    get_mounted_disk().get_mut(&t).as_mut().unwrap().set_head(&head);
    get_mounted_disk().release(t);
}

fn get_root() -> Directory {
    let t = get_mounted_disk().borrow();
    let head = get_mounted_disk().get_mut(&t).as_mut().unwrap().get_head();
    get_mounted_disk().release(t);

    let (pages, size) = deserialize(&head);
    load_directory(&pages, size)
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

    for dir in path {
        let curr_dir = &dirs[dirs.size() - 1];
        let mut dir_entry = None;
        for entry in curr_dir.subdirectories {
            if entry.2 == dir {
                dir_entry = Some(entry);
            }
        }

        if let Some((pages, size, _)) = dir_entry {
            dirs.push(load_directory(&pages, size));
        } else {
            let mut new_dir = Directory {
                subdirectories: Vec::new(),
            };
            let (pages, size) = store_directory(&mut new_dir);
            dirs.push(new_dir);
            let curr_dir = &mut dirs[dirs.size() - 2];
            curr_dir.subdirectories.push((pages, size, dir));
        }
    }
}

pub fn is_directory(path: &String) -> bool {
    true
}

pub fn delete_directory(path: &String) {

}