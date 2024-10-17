use kernel_std::{deserialize, String, Vec};
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
    deserialize(&data)
}

pub fn fs_erase() {

}

pub fn create_directory(path: &String) {

}

pub fn is_directory(path: &String) -> bool {
    true
}

pub fn delete_directory(path: &String) {

}