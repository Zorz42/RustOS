use kernel_test::{kernel_test, kernel_test_mod};
use std::{Rng, String, Vec};
use crate::filesystem::{close_fs, get_fs, init_fs};
use crate::memory_disk::{mount_disk, unmount_disk};
use crate::tests::get_test_disk;

kernel_test_mod!(crate::tests::B0_filesystem);

#[kernel_test]
fn test_fs_erase() {
    for _ in 0..100 {
        get_fs().erase();
    }
}

fn create_random_string(rng: &mut Rng) -> String {
    let len = rng.get(10, 30);
    let mut res = String::new();
    for _ in 0..len {
        res.push((rng.get(48, 127) as u8) as char);
    }
    res
}

#[kernel_test]
fn test_fs_create_delete_exists_file() {
    get_fs().erase();

    let mut existing_files = Vec::new();
    let mut rng = Rng::new(4637894352678);

    for _ in 0..100 {
        if rng.get(0, 2) == 0 || existing_files.size() == 0 {
            // create file
            let file_name = create_random_string(&mut rng);
            get_fs().create_file(&file_name);
            existing_files.push(file_name);
        } else {
            // destroy file
            let file_name = existing_files[rng.get(0, existing_files.size() as u64) as usize].clone();
            get_fs().delete_file(&file_name);
            existing_files.retain(&|x| *x != file_name);
        }

        for _ in 0..10 {
            assert!(get_fs().get_file(&create_random_string(&mut rng)).is_none());
        }

        for file_name in &existing_files {
            assert!(get_fs().get_file(file_name).is_some());
        }
    }
}

#[kernel_test]
fn test_fs_persists() {
    get_fs().erase();

    let mut existing_files = Vec::new();
    let mut rng = Rng::new(4637894352678);

    for _ in 0..100 {
        if rng.get(0, 2) == 0 || existing_files.size() == 0 {
            // create file
            let file_name = create_random_string(&mut rng);
            get_fs().create_file(&file_name);
            existing_files.push(file_name);
        } else {
            // destroy file
            let file_name = existing_files[rng.get(0, existing_files.size() as u64) as usize].clone();
            get_fs().delete_file(&file_name);
            existing_files.retain(&|x| *x != file_name);
        }
        
        close_fs();
        unmount_disk();
        mount_disk(get_test_disk());
        init_fs();

        for _ in 0..10 {
            assert!(get_fs().get_file(&create_random_string(&mut rng)).is_none());
        }

        for file_name in &existing_files {
            assert!(get_fs().get_file(file_name).is_some());
        }
    }
}

fn join(vec: &Vec<String>, c: char) -> String {
    let mut res = String::new();
    for i in vec {
        for c in i {
            res.push(*c);
        }
        res.push(c);
    }
    res.pop();
    res
}

#[kernel_test]
fn test_fs_create_dir() {
    let mut rng = Rng::new(54738524637825);

    for _ in 0..20 {
        let depth = rng.get(1, 20);
        let mut dirs = Vec::new();
        for _ in 0..depth {
            dirs.push(create_random_string(&mut rng));
        }

        let path = join(&dirs, '/');
        get_fs().create_directory(&path);

        close_fs();
        unmount_disk();
        mount_disk(get_test_disk());
        init_fs();

        let mut curr_dirs = Vec::new();
        for i in dirs {
            curr_dirs.push(i);
            assert!(get_fs().get_directory(&join(&curr_dirs, '/')).is_some());
        }
    }
}