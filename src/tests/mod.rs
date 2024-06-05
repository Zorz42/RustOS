use core::arch::asm;
use core::ops::{Deref, DerefMut};
use core::ptr::addr_of;
use kernel_test::all_perf_tests;
use std::{deserialize, print, println, serialize, String, Vec};

use crate::disk::disk::Disk;
use crate::memory::bitset_size_bytes;
use crate::print::{reset_print_color, set_print_color, TextColor};
use crate::timer::get_ticks;
use kernel_test::all_tests;
//use crate::disk::filesystem::get_fs;

mod A0_rand;
mod A1_bitset;
mod A2_paging;
mod A3_heap_tree;
mod A4_malloc;
mod A5_box;
mod A6_vector;
mod A7_string;
mod A8_disk;
//mod A9_memory_disk;
//mod B0_filesystem;

pub trait KernelPerf {
    fn setup() -> Self;
    fn run(&mut self);
    fn teardown(&mut self) {}
}

const TESTDISK_MAGIC_CODE: u32 = 0x61732581;

static mut FREE_SPACE: [u8; bitset_size_bytes(1024 * 8) + 8] = [0; bitset_size_bytes(1024 * 8) + 8];

pub(super) fn get_free_space_addr() -> *mut u8 {
    unsafe { ((FREE_SPACE.as_mut_ptr() as u64 + 7) / 8 * 8) as *mut u8 }
}

static mut TEST_DISK: Option<&'static mut Disk> = None;

pub fn get_test_disk() -> &'static mut Disk {
    unsafe { *TEST_DISK.as_mut().unwrap() }
}

pub fn test_runner(disks: &mut Vec<&'static mut Disk>) {
    let mut test_disk = None;
    for disk in disks {
        let first_sector = disk.read(0);
        let magic = ((first_sector[511] as u32) << 0) + ((first_sector[510] as u32) << 8) + ((first_sector[509] as u32) << 16) + ((first_sector[508] as u32) << 24);

        if magic == TESTDISK_MAGIC_CODE {
            let addr = addr_of!(**disk);
            test_disk = Some(unsafe { addr as *mut Disk });
        }
    }

    if let Some(test_disk) = test_disk {
        unsafe {
            TEST_DISK = Some(&mut *test_disk);
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
        let width = max_length - name.len();
        for _ in 0..width {
            print!(" ");
        }
        set_print_color(TextColor::LightGray, TextColor::Black);
        print!("[");
        set_print_color(TextColor::LightGreen, TextColor::Black);
        print!("OK");
        set_print_color(TextColor::LightGray, TextColor::Black);
        print!("] ");
        set_print_color(TextColor::LightGray, TextColor::Black);
        println!("{}ms", end_time - start_time);
    }

    println!();
    reset_print_color();
}

/*pub fn perf_test_runner() {
    set_print_color(TextColor::Pink, TextColor::Black);
    all_perf_tests!();
    println!();
    reset_print_color();
}*/

/*const PERF_COOLDOWN_DURATION_MS: u32 = 1000;
const PERF_WARMUP_DURATION_MS: u32 = 1000;
const PERF_TEST_DURATION_MS: u32 = 3000;
const PERF_FILE: &str = "perf.data";
const PERF_FILE_SAVE: &str = "perf-new.data";

fn get_perf_data(name: &str) -> Option<f32> {
    let file = get_fs().get_file(&String::from(PERF_FILE))?;
    let vec = deserialize::<Vec<(String, f32)>>(&file.read());

    for (perf_name, val) in vec {
        if perf_name.as_str() == name {
            return Some(val);
        }
    }
    None
}

fn save_perf_data(name: &str, val: f32) {
    let file = if let Some(file) = get_fs().get_file(&String::from(PERF_FILE_SAVE)) {
        file
    } else {
        get_fs().create_file(&String::from(PERF_FILE_SAVE))
    };

    let mut vec = if file.read().size() == 0 {
        Vec::new()
    } else {
        deserialize::<Vec<(String, f32)>>(&file.read())
    };
    let mut saved = false;
    for (perf_name, perf_val) in &mut vec {
        if perf_name.as_str() == name {
            *perf_val = val;
            assert!(!saved);
            saved = true;
        }
    }

    if !saved {
        vec.push((String::from(name), val));
    }

    file.write(&serialize(&mut vec));
}

fn run_perf_test<T: KernelPerf>(name: &str) {
    let mut test_struct = T::setup();

    set_print_color(TextColor::DarkGray, TextColor::Black);
    print!("Benchmarking");
    set_print_color(TextColor::LightCyan, TextColor::Black);
    print!(" {name}");

    // cooldown
    let start_time = get_ticks();
    while get_ticks() - start_time < PERF_COOLDOWN_DURATION_MS {
        unsafe {
            asm!("hlt");
        }
    }

    // warmup
    let start_time = get_ticks();
    while get_ticks() - start_time < PERF_WARMUP_DURATION_MS {
        test_struct.run();
    }

    // actual measurement
    let start_time = get_ticks();
    let mut count = 0;
    while get_ticks() - start_time < PERF_TEST_DURATION_MS {
        count += 1;
        test_struct.run();
    }
    let duration = get_ticks() - start_time;
    test_struct.teardown();

    if name.len() > 39 {
        panic!("Too long test name");
    }

    let num_spaces = 40 - name.len();
    for _ in 0..num_spaces {
        print!(" ");
    }

    let perf_ms = duration as f32 / count as f32;

    let saved_perf_ms = get_perf_data(name);

    set_print_color(TextColor::White, TextColor::Black);
    print!("{:10.6}", perf_ms);
    set_print_color(TextColor::LightGray, TextColor::Black);
    print!("ms");

    if let Some(saved_perf_ms) = saved_perf_ms {
        let percent = perf_ms / saved_perf_ms * 100.0 - 100.0;

        print!(" / ");
        set_print_color(TextColor::White, TextColor::Black);
        print!("{:10.6}", saved_perf_ms);
        set_print_color(TextColor::LightGray, TextColor::Black);
        print!("ms");

        if percent < -10.0 {
            set_print_color(TextColor::LightGreen, TextColor::Black);
        } else if percent < 10.0 {
            set_print_color(TextColor::LightGray, TextColor::Black);
        } else {
            set_print_color(TextColor::LightRed, TextColor::Black);
        }

        print!("       {:.1}%", percent);
    }
    println!();

    save_perf_data(name, perf_ms);
}*/
