use std::Vec;

use crate::disk::Disk;
use crate::timer::get_ticks;

mod A0_rand;
mod A1_utils;
mod A2_bitset;
mod A3_paging;
mod A4_heap_tree;
mod A5_malloc;
mod A6_box;
mod A7_vector;
mod A8_disk;
mod A9_memory_disk;
mod B0_filesystem;

const TESTDISK_MAGIC_CODE: u32 = 0x61732581;

static mut FREE_SPACE: [u8; 1032] = [0; 1032];

pub(super) fn get_free_space_addr() -> *mut u8 {
    unsafe { ((FREE_SPACE.as_mut_ptr() as u64 + 7) / 8 * 8) as *mut u8 }
}

static mut TEST_DISK: Option<Disk> = None;

pub fn get_test_disk() -> Disk {
    unsafe { TEST_DISK.as_ref().unwrap().clone() }
}

pub fn test_runner(disks: &Vec<Disk>) {
    use kernel_test::all_tests;

    use crate::print::{reset_print_color, set_print_color, TextColor};
    use crate::{print, println};

    let mut test_disk = None;
    for disk in disks {
        let first_sector = disk.read(0);
        let magic = ((first_sector[511] as u32) << 0) + ((first_sector[510] as u32) << 8) + ((first_sector[509] as u32) << 16) + ((first_sector[508] as u32) << 24);

        if magic == TESTDISK_MAGIC_CODE {
            test_disk = Some(disk.clone());
        }
    }

    if let Some(test_disk) = test_disk {
        unsafe {
            TEST_DISK = Some(test_disk);
        }
    } else {
        panic!("Test disk not found");
    }

    let tests = all_tests!();

    let mut max_length = 0;

    for (_, name) in tests {
        max_length = max_length.max((name.len() + 9) / 10 * 10);
    }

    set_print_color(TextColor::Pink, TextColor::Black);
    println!("Running {} tests", tests.len());
    for (test_fn, name) in tests {
        set_print_color(TextColor::DarkGray, TextColor::Black);
        print!("Testing");
        set_print_color(TextColor::LightCyan, TextColor::Black);
        print!(" {name}");
        let start_time = get_ticks();
        test_fn();
        let end_time = get_ticks();
        set_print_color(TextColor::LightGreen, TextColor::Black);
        let width = max_length - name.len();
        for _ in 0..width {
            print!(" ");
        }
        print!("[OK] ");
        set_print_color(TextColor::LightGray, TextColor::Black);
        println!("{}ms", end_time - start_time);
    }

    reset_print_color();
}
