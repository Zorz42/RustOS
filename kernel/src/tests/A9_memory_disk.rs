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
