use kernel_test::{kernel_test, kernel_test_mod};
use crate::memory_disk::{get_mounted_disk, mount_disk, DiskBox};
use crate::tests::get_test_disk;
use std::{Vec, serialize, deserialize, Rng};
kernel_test_mod!(crate::tests::A9_memory_disk);

#[kernel_test]
fn test_disk_mount_erase() {
    mount_disk(get_test_disk());
    get_mounted_disk().erase();
}

#[kernel_test]
fn test_disk_create_destroy() {
    let disk = get_mounted_disk();

    for i in 0..1000 {
        let id = disk.create();
        disk.destroy(id);
    }
}

#[kernel_test]
fn test_disk_create_destroy_multiple() {
    let disk = get_mounted_disk();

    for i in 0..100 {
        let mut arr = [0; 100];
        for j in 0..100 {
            arr[j] = disk.create();
        }

        for id in arr {
            disk.destroy(id);
        }
    }
}

#[kernel_test]
fn test_disk_save_load() {
    let disk = get_mounted_disk();
    let mut rng = Rng::new(436752832345);

    const ARRAY_REPEAT_VALUE: Option<Vec<u8>> = None;

    let mut arr = [0; 1000];
    let mut vecs = [ARRAY_REPEAT_VALUE; 1000];

    for i in 0..1000 {
        let size = rng.get(0, 100);
        vecs[i] = Some(Vec::new());
        for _ in 0..size {
            if let Some(vec) = &mut vecs[i] {
                vec.push(rng.get(0, 256) as u8);
            }
        }
        
        arr[i] = disk.create();
        if let Some(vec) = &vecs[i] {
            disk.save(arr[i], &vec);
        }
    }

    for i in 0..1000 {
        if let Some(vec) = &vecs[i] {
            let vec2 = disk.load(arr[i]);
            assert!(*vec == vec2);
        }
    }

    for i in 0..1000 {
        disk.destroy(arr[i]);
    }
}

#[kernel_test]
fn test_disk_save_load_big() {
    let disk = get_mounted_disk();
    let mut rng = Rng::new(436752832345);

    const ARRAY_REPEAT_VALUE: Option<Vec<u8>> = None;

    let mut arr = [0; 10];
    let mut vecs = [ARRAY_REPEAT_VALUE; 10];

    for i in 0..10 {
        let size = rng.get(0, 100000);
        vecs[i] = Some(Vec::new());
        for _ in 0..size {
            if let Some(vec) = &mut vecs[i] {
                vec.push(rng.get(0, 256) as u8);
            }
        }

        arr[i] = disk.create();
        if let Some(vec) = &vecs[i] {
            disk.save(arr[i], &vec);
        }
    }

    for i in 0..10 {
        if let Some(vec) = &vecs[i] {
            assert!(*vec == disk.load(arr[i]));
        }
    }

    for i in 0..10 {
        disk.destroy(arr[i]);
    }
}

#[kernel_test]
fn test_diskbox() {
    let mut rng = Rng::new(5643728523);
    
    for _ in 0..20 {
        let len = rng.get(0, 40) as usize;
        let mut vec = Vec::new();
        
        for _ in 0..len {
            vec.push(rng.get(0, 1u64 << 63));
        }
        
        let mut vec1 = Vec::new();
        for i in &vec {
            vec1.push(DiskBox::new(*i));
        }
        let data = serialize(&vec1);
        vec1 = deserialize(&data);
        
        for i in 0..len {
            assert_eq!(*vec1[i].get(), vec[i]);
        }
        
        for i in vec1 {
            DiskBox::delete(i);
        }
    }
}
