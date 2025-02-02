use kernel_test::all_perf_tests;
use kernel_std::{deserialize, print, println, serialize, Mutable, String, Vec};

use crate::disk::disk::Disk;
use kernel_std::bitset_size_bytes;
use crate::print::{reset_print_color, set_print_color};
use crate::timer::get_ticks;
use kernel_test::all_tests;
use crate::disk::filesystem::{read_file, write_to_file};
use crate::ROOT_MAGIC;
use crate::text_renderer::TextColor;

mod A0_rand;
mod A1_bitset;
mod A2_paging;
mod A3_malloc;
mod A4_box;
mod A5_vector;
mod A6_string;
mod A7_disk;
mod A8_memory_disk;
mod A9_filesystem;
mod B0_scheduler;

pub trait KernelPerf {
    fn setup() -> Self;
    fn run(&mut self);
    fn teardown(&mut self) {}
}

const TESTDISK_MAGIC_CODE: u32 = 0x61732581;

static FREE_SPACE: Mutable<[u8; bitset_size_bytes(1024 * 8) + 8]> = Mutable::new([0; bitset_size_bytes(1024 * 8) + 8]);

pub(super) fn get_free_space_addr() -> *mut u8 {
    let t = FREE_SPACE.borrow();
    let res = ((FREE_SPACE.get_mut(&t).as_mut_ptr() as u64 + 7) / 8 * 8) as *mut u8;
    FREE_SPACE.release(t);
    res
}

static TEST_DISK: Mutable<Option<Disk>> = Mutable::new(None);

pub fn get_test_disk() -> &'static Mutable<Option<Disk>> {
    &TEST_DISK
}

pub fn test_runner(disks: &mut Vec<Disk>) {

    let mut test_disk = None;
    for disk in disks {
        let first_sector = disk.read(0);
        let magic = ((first_sector[511] as u32) << 0) + ((first_sector[510] as u32) << 8) + ((first_sector[509] as u32) << 16) + ((first_sector[508] as u32) << 24);

        if magic != ROOT_MAGIC {
            test_disk = Some(disk.clone());
        }
    }

    if test_disk.is_none() {
        panic!("Test disk not found");
    }

    let t = TEST_DISK.borrow();
    *TEST_DISK.get_mut(&t) = test_disk;
    TEST_DISK.release(t);

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

pub fn perf_test_runner() {
    set_print_color(TextColor::Pink, TextColor::Black);
    all_perf_tests!();
    println!();
    reset_print_color();
}

const PERF_TEST_MAX_ITERATIONS: u64 = 10000;
const PERF_FILE: &str = "perf.data";
const PERF_FILE_SAVE: &str = "perf-new.data";

fn get_perf_data(name: &str) -> Option<f64> {
    let file = read_file(&String::from(PERF_FILE))?;
    let vec = deserialize::<Vec<(String, f64)>>(&file);

    for (perf_name, val) in vec {
        if perf_name.as_str() == name {
            return Some(val);
        }
    }
    None
}

fn save_perf_data(name: &str, val: f64) {
    let mut vec: Vec<(String, f64)> = Vec::new();
    if let Some(data) = read_file(&String::from(PERF_FILE_SAVE)) {
        vec = deserialize::<Vec<(String, f64)>>(&data);
    }

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

    let data = serialize(&mut vec);
    write_to_file(&String::from(PERF_FILE_SAVE), &data);
}

fn run_perf_test<T: KernelPerf>(name: &str) {
    let mut test_struct = T::setup();

    set_print_color(TextColor::DarkGray, TextColor::Black);
    print!("Benchmarking");
    set_print_color(TextColor::LightCyan, TextColor::Black);
    print!(" {name}");

    let mut total_ms = 0;
    let mut iterations = 0;
    let start_time = get_ticks();
    for _ in 0..PERF_TEST_MAX_ITERATIONS {
        let start_ms = get_ticks();
        test_struct.run();
        let end_ms = get_ticks();
        // add ms
        total_ms += end_ms - start_ms;
        iterations += 1;
        if get_ticks() - start_time > 3000 {
            break;
        }
    }
    test_struct.teardown();

    if name.len() > 39 {
        panic!("Too long test name");
    }

    let num_spaces = 40 - name.len();
    for _ in 0..num_spaces {
        print!(" ");
    }

    let perf_ms = total_ms as f64 / iterations as f64;

    let saved_perf_ms = get_perf_data(name);

    set_print_color(TextColor::White, TextColor::Black);
    print!("{:.5}", perf_ms);
    set_print_color(TextColor::LightGray, TextColor::Black);
    print!("ms");

    if let Some(saved_perf_ms) = saved_perf_ms{
        let percent = perf_ms / saved_perf_ms * 100.0 - 100.0;

        print!(" / ");
        set_print_color(TextColor::White, TextColor::Black);
        print!("{:.5}", saved_perf_ms);
        set_print_color(TextColor::LightGray, TextColor::Black);
        print!("ms");

        if percent < -3.0 {
            set_print_color(TextColor::LightGreen, TextColor::Black);
        } else if percent < 3.0 {
            set_print_color(TextColor::LightGray, TextColor::Black);
        } else {
            set_print_color(TextColor::LightRed, TextColor::Black);
        }

        print!("       {:.1}%", percent);
    }
    println!();

    save_perf_data(name, perf_ms);
}
