use crate::disk::memory_disk::{get_mounted_disk, mount_disk, unmount_disk};
use crate::tests::{get_test_disk, KernelPerf};
use kernel_test::{kernel_perf, kernel_test, kernel_test_mod};
use kernel_std::{print, println, Rng, String, Vec};
use crate::disk::filesystem::{fs_erase, create_directory, is_directory, delete_directory, write_to_file, delete_file, is_file, read_file};

kernel_test_mod!(crate::tests::B0_filesystem);

#[kernel_test]
fn test_fs_erase() {
    let t = get_mounted_disk().borrow();
    get_mounted_disk().get_mut(&t).as_mut().unwrap().erase();
    get_mounted_disk().release(t);
    
    for _ in 0..100 {
        fs_erase();
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
fn test_fs_create_delete_check_dir() {
    fs_erase();

    let t2 = get_test_disk().borrow();
    let test_disk = get_test_disk().get_mut(&t2).as_mut().unwrap();

    let mut rng = Rng::new(54738524637825);

    for _ in 0..10 {
        let depth = rng.get(1, 10);
        let mut dirs = Vec::new();
        for _ in 0..depth {
            dirs.push(create_random_string(&mut rng));
        }

        let path = join(&dirs, '/');

        create_directory(&path);

        unmount_disk();
        mount_disk(test_disk);

        let mut curr_dirs = Vec::new();
        for i in &dirs {
            curr_dirs.push(i.clone());
            assert!(is_directory(&join(&curr_dirs, '/')));

            curr_dirs.push(create_random_string(&mut rng));
            for i in 0..20 {
                assert!(!is_directory(&join(&curr_dirs, '/')));
            }
            curr_dirs.pop();
        }

        let mut curr_dirs = Vec::new();
        for i in &dirs {
            curr_dirs.push(i.clone());
            assert!(is_directory(&join(&curr_dirs, '/')));
        }

        assert!(is_directory(&path));

        delete_directory(&dirs[0]);

        let mut curr_dirs = Vec::new();
        for i in &dirs {
            curr_dirs.push(i.clone());
            assert!(!is_directory(&join(&curr_dirs, '/')));
        }

        assert!(!is_directory(&path));
    }

    get_test_disk().release(t2);
}

#[kernel_test]
fn test_fs_create_delete_exists_file() {
    fs_erase();

    let mut existing_files = Vec::new();
    let mut rng = Rng::new(4637894352678);

    for _ in 0..100 {
        if rng.get(0, 2) == 0 || existing_files.size() == 0 {
            // create file
            let file_name = create_random_string(&mut rng);
            write_to_file(&file_name, &Vec::new());
            existing_files.push(file_name);
        } else {
            // destroy file
            let file_name = existing_files[rng.get(0, existing_files.size() as u64) as usize].clone();
            delete_file(&file_name);
            existing_files.retain(&|x| *x != file_name);
            assert!(!is_file(&file_name));
        }

        for _ in 0..10 {
            assert!(!is_file(&create_random_string(&mut rng)));
        }

        for file_name in &existing_files {
            assert!(is_file(file_name));
        }
    }
}

#[kernel_test]
fn test_fs_persists() {
    fs_erase();

    let t2 = get_test_disk().borrow();
    let test_disk = get_test_disk().get_mut(&t2).as_mut().unwrap();

    let mut existing_files = Vec::new();
    let mut rng = Rng::new(4637894352678);

    for _ in 0..20 {
        if rng.get(0, 2) == 0 || existing_files.size() == 0 {
            // create file
            let file_name = create_random_string(&mut rng);
            write_to_file(&file_name, &Vec::new());
            existing_files.push(file_name);
        } else {
            // destroy file
            let file_name = existing_files[rng.get(0, existing_files.size() as u64) as usize].clone();
            delete_file(&file_name);
            existing_files.retain(&|x| *x != file_name);
        }

        unmount_disk();
        mount_disk(test_disk);

        for _ in 0..10 {
            assert!(!is_file(&create_random_string(&mut rng)));
        }

        for file_name in &existing_files {
            assert!(is_file(file_name));
        }
    }
    get_test_disk().release(t2);
}

#[kernel_test]
fn test_fs_read_write_file() {
    let mut rng = Rng::new(54738524637825);
    let mut vec = Vec::new();

    create_directory(&String::from("vec"));

    for i in 0..20 {
        let mut data = Vec::new();
        let len = rng.get(0, 10000);
        for _ in 0..len {
            data.push(rng.get(0, 1 << 8) as u8);
        }
        let mut file_name = String::from("vec/");
        file_name.push(('A' as u8 + i as u8) as char);
        write_to_file(&file_name, &data);
        vec.push(data);
    }

    for i in 0..20 {
        let mut file_name = String::from("vec/");
        file_name.push(('A' as u8 + i as u8) as char);
        let data = read_file(&file_name).unwrap();
        assert!(data == vec[i]);
    }

    // try two multisector files
    let mut data1 = Vec::new();
    for _ in 0..5000 {
        data1.push(rng.get(0, 1 << 8) as u8);
    }
    write_to_file(&String::from("big_file1"), &data1);

    let mut data2 = Vec::new();
    for _ in 0..5000 {
        data2.push(rng.get(0, 1 << 8) as u8);
    }
    write_to_file(&String::from("big_file2"), &data2);

    let t2 = get_test_disk().borrow();
    let test_disk = get_test_disk().get_mut(&t2).as_mut().unwrap();
    unmount_disk();
    mount_disk(test_disk);
    get_test_disk().release(t2);

    let data1_read = read_file(&String::from("big_file1")).unwrap();
    let data2_read = read_file(&String::from("big_file2")).unwrap();

    assert!(data1 == data1_read);
    assert!(data2 == data2_read);

    // try two multisector files again
    let mut data1 = Vec::new();
    for _ in 0..10000 {
        data1.push(rng.get(0, 1 << 8) as u8);
    }
    write_to_file(&String::from("big_file1"), &data1);

    let mut data2 = Vec::new();
    for _ in 0..10000 {
        data2.push(rng.get(0, 1 << 8) as u8);
    }
    write_to_file(&String::from("big_file2"), &data2);

    let t2 = get_test_disk().borrow();
    let test_disk = get_test_disk().get_mut(&t2).as_mut().unwrap();
    unmount_disk();
    mount_disk(test_disk);
    get_test_disk().release(t2);

    let data1_read = read_file(&String::from("big_file1")).unwrap();
    let data2_read = read_file(&String::from("big_file2")).unwrap();

    assert!(data1 == data1_read);
    assert!(data2 == data2_read);

    let mut data1 = Vec::new();
    for _ in 0..5000 {
        data1.push(rng.get(0, 1 << 8) as u8);
    }
    write_to_file(&String::from("big_file1"), &data1);

    let mut data2 = Vec::new();
    for _ in 0..5000 {
        data2.push(rng.get(0, 1 << 8) as u8);
    }
    write_to_file(&String::from("big_file2"), &data2);

    let t2 = get_test_disk().borrow();
    let test_disk = get_test_disk().get_mut(&t2).as_mut().unwrap();
    unmount_disk();
    mount_disk(test_disk);
    get_test_disk().release(t2);

    let data1_read = read_file(&String::from("big_file1")).unwrap();
    let data2_read = read_file(&String::from("big_file2")).unwrap();

    assert!(data1 == data1_read);
    assert!(data2 == data2_read);
}

#[kernel_test]
fn test_fs_many_writes() {
    for _ in 0..1000 {
        write_to_file(&String::from("test_file"), &Vec::new());
    }

    for _ in 0..1000 {
        write_to_file(&String::from("test_file"), &Vec::new());
        delete_file(&String::from("test_file"));
    }
}

#[kernel_test]
fn test_fs_random_files_persists() {
    let mut rng = Rng::new(54738524637825);
    let mut file_data: Vec<Vec<u8>> = Vec::new();
    let mut file_names: Vec<String> = Vec::new();
    for i in 0..10 {
        let mut data = Vec::new();
        let len = rng.get(0, 1000);
        for _ in 0..len {
            data.push(rng.get(0, 1 << 8) as u8);
        }
        let mut file_name = String::from("vec/");
        file_name.push(('A' as u8 + i as u8) as char);
        write_to_file(&file_name, &data);
        file_data.push(data);
        file_names.push(file_name);

        let t2 = get_test_disk().borrow();
        let test_disk = get_test_disk().get_mut(&t2).as_mut().unwrap();
        unmount_disk();
        mount_disk(test_disk);
        get_test_disk().release(t2);
    }

    for i in 0..20 {
        let idx = rng.get(0, file_data.size() as u64) as usize;
        let len = rng.get(0, 1000);
        let mut data = Vec::new();
        for _ in 0..len {
            data.push(rng.get(0, 1 << 8) as u8);
        }
        write_to_file(&file_names[idx], &data);
        file_data[idx] = data;

        let t2 = get_test_disk().borrow();
        let test_disk = get_test_disk().get_mut(&t2).as_mut().unwrap();
        unmount_disk();
        mount_disk(test_disk);
        get_test_disk().release(t2);

        for i in 0..file_data.size() {
            let data = read_file(&file_names[i]).unwrap();
            assert!(data == file_data[i]);
        }
    }
}

#[kernel_perf]
struct PerfCreateDeleteFile {}

impl KernelPerf for PerfCreateDeleteFile {
    fn setup() -> Self {
        Self {}
    }

    fn run(&mut self) {
        write_to_file(&String::from("test_file"), &Vec::new());
        delete_file(&String::from("test_file"));
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
            write_to_file(&file_name, &Vec::new());
        }

        for i in 0..100 {
            let mut file_name = String::from("test_file");
            file_name.push(('0' as u8 + (i as u8 / 10)) as char);
            file_name.push(('0' as u8 + (i as u8 % 10)) as char);
            delete_file(&file_name);
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
        let mut vec = Vec::new();
        for _ in 0..10 {
            vec.push(111);
        }
        write_to_file(&String::from("test_file"), &vec);
        delete_file(&String::from("test_file"));
    }
}

#[kernel_perf]
struct PerfWriteFileBig {}

impl KernelPerf for PerfWriteFileBig {
    fn setup() -> Self {
        Self {}
    }

    fn run(&mut self) {
        let mut vec = Vec::new();
        for _ in 0..100000 {
            vec.push(111);
        }
        write_to_file(&String::from("test_file"), &vec);
        delete_file(&String::from("test_file"));
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
            write_to_file(&file_name, &vec);
        }

        for i in 0..100 {
            let mut file_name = String::from("test_file");
            file_name.push(('0' as u8 + (i as u8 / 10)) as char);
            file_name.push(('0' as u8 + (i as u8 % 10)) as char);
            delete_file(&file_name);
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
            write_to_file(&file_name, &vec);
        }

        for i in 0..50 {
            let mut file_name = String::from("test_file");
            file_name.push(('0' as u8 + (i as u8 / 10)) as char);
            file_name.push(('0' as u8 + (i as u8 % 10)) as char);
            delete_file(&file_name);
        }
    }
}
