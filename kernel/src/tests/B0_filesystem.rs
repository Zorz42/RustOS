use crate::disk::filesystem::{close_fs, get_fs, init_fs};
use crate::disk::memory_disk::{get_mounted_disk, mount_disk, unmount_disk};
use crate::tests::{get_test_disk, KernelPerf};
use kernel_test::{kernel_perf, kernel_test, kernel_test_mod};
use std::{Rng, String, Vec};

kernel_test_mod!(crate::tests::B0_filesystem);

#[kernel_test]
fn test_fs_erase() {
    let t = get_mounted_disk().borrow();
    get_mounted_disk().get_mut(&t).as_mut().unwrap().erase();
    get_mounted_disk().release(t);

    init_fs();
    
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

    for _ in 0..20 {
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

    for _ in 0..10 {
        let depth = rng.get(1, 10);
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
        for i in dirs.clone() {
            curr_dirs.push(i);
            assert!(get_fs().get_directory(&join(&curr_dirs, '/')).is_some());
        }

        get_fs().delete_directory(&dirs[0]);

        let mut curr_dirs = Vec::new();
        for i in dirs.clone() {
            curr_dirs.push(i);
            assert!(get_fs().get_directory(&join(&curr_dirs, '/')).is_none());
        }
    }
}

#[kernel_test]
fn test_fs_read_write_file() {
    let mut rng = Rng::new(54738524637825);
    let mut vec = Vec::new();

    get_fs().create_directory(&String::from("vec"));

    for i in 0..20 {
        let mut data = Vec::new();
        let len = rng.get(0, 10000);
        for _ in 0..len {
            data.push(rng.get(0, 1 << 8) as u8);
        }
        let mut file_name = String::from("vec/");
        file_name.push(('A' as u8 + i as u8) as char);
        let file = get_fs().create_file(&file_name);
        file.write(&data);
        vec.push(data);
    }

    for i in 0..20 {
        let mut file_name = String::from("vec/");
        file_name.push(('A' as u8 + i as u8) as char);
        let file = get_fs().get_file(&file_name).unwrap();
        let data = file.read();
        assert!(data == vec[i]);
    }
}

#[kernel_perf]
struct PerfCreateDeleteFile {}

impl KernelPerf for PerfCreateDeleteFile {
    fn setup() -> Self {
        Self {}
    }

    fn run(&mut self) {
        get_fs().create_file(&String::from("test_file"));
        get_fs().delete_file(&String::from("test_file"));
    }
}

#[kernel_perf]
struct PerfCreateDeleteFile100 {}

impl KernelPerf for PerfCreateDeleteFile100 {
    fn setup() -> Self {
        Self {}
    }

    fn run(&mut self) {
        for i in 0..100 {
            let mut file_name = String::from("test_file");
            file_name.push(('0' as u8 + (i as u8 / 10)) as char);
            file_name.push(('0' as u8 + (i as u8 % 10)) as char);
            get_fs().create_file(&file_name);
        }

        for i in 0..100 {
            let mut file_name = String::from("test_file");
            file_name.push(('0' as u8 + (i as u8 / 10)) as char);
            file_name.push(('0' as u8 + (i as u8 % 10)) as char);
            get_fs().delete_file(&file_name);
        }
    }
}

#[kernel_perf]
struct PerfWriteFile {}

impl KernelPerf for PerfWriteFile {
    fn setup() -> Self {
        Self {}
    }

    fn run(&mut self) {
        let file = get_fs().create_file(&String::from("test_file"));
        let mut vec = Vec::new();
        for _ in 0..10 {
            vec.push(111);
        }
        file.write(&vec);
        get_fs().delete_file(&String::from("test_file"));
    }
}

#[kernel_perf]
struct PerfWriteFileBig {}

impl KernelPerf for PerfWriteFileBig {
    fn setup() -> Self {
        Self {}
    }

    fn run(&mut self) {
        let file = get_fs().create_file(&String::from("test_file"));
        let mut vec = Vec::new();
        for _ in 0..100000 {
            vec.push(111);
        }
        file.write(&vec);
        get_fs().delete_file(&String::from("test_file"));
    }
}

#[kernel_perf]
struct PerfWriteFile100 {}

impl KernelPerf for PerfWriteFile100 {
    fn setup() -> Self {
        Self {}
    }

    fn run(&mut self) {
        let mut vec = Vec::new();
        for _ in 0..10 {
            vec.push(111);
        }
        for i in 0..100 {
            let mut file_name = String::from("test_file");
            file_name.push(('0' as u8 + (i as u8 / 10)) as char);
            file_name.push(('0' as u8 + (i as u8 % 10)) as char);
            let file = get_fs().create_file(&file_name);
            file.write(&vec);
        }

        for i in 0..100 {
            let mut file_name = String::from("test_file");
            file_name.push(('0' as u8 + (i as u8 / 10)) as char);
            file_name.push(('0' as u8 + (i as u8 % 10)) as char);
            get_fs().delete_file(&file_name);
        }
    }
}

#[kernel_perf]
struct PerfWriteBigFile50 {}

impl KernelPerf for PerfWriteBigFile50 {
    fn setup() -> Self {
        Self {}
    }

    fn run(&mut self) {
        let mut vec = Vec::new();
        for _ in 0..10000 {
            vec.push(111);
        }
        for i in 0..50 {
            let mut file_name = String::from("test_file");
            file_name.push(('0' as u8 + (i as u8 / 10)) as char);
            file_name.push(('0' as u8 + (i as u8 % 10)) as char);
            let file = get_fs().create_file(&file_name);
            file.write(&vec);
        }

        for i in 0..50 {
            let mut file_name = String::from("test_file");
            file_name.push(('0' as u8 + (i as u8 / 10)) as char);
            file_name.push(('0' as u8 + (i as u8 % 10)) as char);
            get_fs().delete_file(&file_name);
        }
    }
}
