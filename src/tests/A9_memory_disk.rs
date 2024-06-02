use crate::memory::{DISK_OFFSET, PAGE_SIZE};
use crate::disk::memory_disk::{get_mounted_disk, mount_disk, unmount_disk, DiskBox};
use crate::tests::get_test_disk;
use kernel_test::{kernel_test, kernel_test_mod};
use std::{deserialize, serialize, Rng, Vec};
use crate::{print, println};
kernel_test_mod!(crate::tests::A9_memory_disk);

#[kernel_test]
fn test_disk_mount_erase() {
    mount_disk(get_test_disk());
    get_mounted_disk().erase();
}

#[kernel_test]
fn test_disk_persists() {
    let mut rng = Rng::new(56437285922);
    for _ in 0..20 {
        let page = get_mounted_disk().alloc_page();
        let addr = (DISK_OFFSET + PAGE_SIZE * page as u64) as *mut u8;
        let mut data = [0; PAGE_SIZE as usize];
        for i in 0..PAGE_SIZE {
            data[i as usize] = rng.get(0, 1 << 8) as u8;
            unsafe {
                *addr.add(i as usize) = data[i as usize];
            }
        }
        unmount_disk();
        mount_disk(get_test_disk());
        for i in 0..PAGE_SIZE {
            unsafe {
                assert_eq!(*addr.add(i as usize), data[i as usize]);
            }
        }
        get_mounted_disk().free_page(page);
    }
}

#[kernel_test]
fn test_disk_head_persists() {
    let mut rng = Rng::new(7865436873);

    for _ in 0..20 {
        let len = rng.get(0, 40) as usize;
        let mut vec = Vec::new();

        for _ in 0..len {
            vec.push(rng.get(0, 1u64 << 8) as u8);
        }

        get_mounted_disk().set_head(&vec);

        unmount_disk();
        mount_disk(get_test_disk());

        let vec1 = get_mounted_disk().get_head();
        
        assert!(vec == vec1);
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
        
        let data = serialize(&mut vec1);

        vec1 = deserialize(&data);
        
        for i in 0..len {
            assert_eq!(*vec1[i].get(), vec[i]);
        }
        
        for i in vec1 {
            DiskBox::delete(i);
        }
    }
}

#[kernel_test]
fn test_diskbox_persists() {
    let mut rng = Rng::new(5643728235352);

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
        let data = serialize(&mut vec1);
        vec1 = deserialize(&data);

        unmount_disk();
        mount_disk(get_test_disk());

        for i in 0..len {
            assert_eq!(*vec1[i].get(), vec[i]);
        }

        for i in vec1 {
            DiskBox::delete(i);
        }
    }
}