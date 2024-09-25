use crate::tests::get_test_disk;
use kernel_test::{kernel_test, kernel_test_mod};
use std::{print, println, Rng, Vec};

kernel_test_mod!(crate::tests::A8_disk);

#[kernel_test]
fn test_disk_write() {
    let mut rng = Rng::new(5324275428);
    let mut data = [0; 512];
    let test_disk = get_test_disk();

    for _ in 0..1000 {
        for i in 0..512 {
            data[i] = rng.get(0, 1 << 8) as u8;
        }

        let sector = rng.get(1, test_disk.size() as u64) as usize;
        test_disk.write(sector, &data);
    }
}

#[kernel_test]
fn test_disk_read() {
    let mut rng = Rng::new(45673543654);
    let mut data = [0; 512];
    let test_disk = get_test_disk();

    for _ in 0..1000 {
        let sector = rng.get(0, test_disk.size() as u64) as usize;
        data = test_disk.read(sector);
    }
}

#[kernel_test]
fn test_disk_read_write() {
    let mut rng = Rng::new(679854467982);
    let mut data = [0; 512];
    let test_disk = get_test_disk();

    for i in 0..1000 {
        for j in 0..512 {
            data[j] = rng.get(0, 1 << 8) as u8;
        }

        let sector = rng.get(1, test_disk.size() as u64) as usize;
        test_disk.write(sector, &data);
        let read_data = test_disk.read(sector);
        assert_eq!(data, read_data);
    }
}

#[kernel_test]
fn test_disk_read_write_shuffled() {
    let mut rng = Rng::new(679854467982);
    let mut data = Vec::new();
    for i in 0..100 {
        let mut data1 = [0; 512];
        for j in 0..512 {
            data1[j] = rng.get(0, 1 << 8) as u8;
        }
        data.push(data1);
    }
    let mut is_written = [false; 100];
    let test_disk = get_test_disk();

    for _ in 0..1000 {
        let i = rng.get(0, 100) as usize;
        let sector = i + 1;
        if is_written[i] {
            let read_data = test_disk.read(sector);
            assert_eq!(data[i], read_data);
        } else {
            test_disk.write(sector, &data[i]);
            is_written[i] = true;
        }
    }
}