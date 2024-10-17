use core::arch::asm;
use core::ops::{Deref, DerefMut};
use core::ptr::addr_of;
use kernel_test::all_perf_tests;
use kernel_std::{deserialize, print, println, serialize, Mutable, String, Vec};

use crate::disk::disk::Disk;
use crate::memory::bitset_size_bytes;
use crate::print::{reset_print_color, set_print_color};
use crate::timer::get_ticks;
use kernel_test::all_tests;
use crate::riscv::get_instruction_count;
use crate::ROOT_MAGIC;
use crate::text_renderer::TextColor;

mod A0_rand;
mod A1_bitset;
mod A2_paging;
mod A3_heap_tree;
mod A4_malloc;
mod A5_box;
mod A6_vector;
mod A7_string;
mod A8_disk;
mod A9_memory_disk;
mod B0_filesystem;

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

const PERF_TEST_ITERATIONS: u64 = 100;
const PERF_FILE: &str = "perf.data";
const PERF_FILE_SAVE: &str = "perf-new.data";

/*fn get_perf_data(name: &str) -> Option<u64> {
    let file = get_fs().get_file(&String::from(PERF_FILE))?;
    let vec = deserialize::<Vec<(String, u64)>>(&file.read());

    for (perf_name, val) in vec {
        if perf_name.as_str() == name {
            return Some(val);
        }
    }
    None
}

fn save_perf_data(name: &str, val: u64) {
    let file = if let Some(file) = get_fs().get_file(&String::from(PERF_FILE_SAVE)) {
        file
    } else {
        get_fs().create_file(&String::from(PERF_FILE_SAVE))
    };

    let mut vec = if file.read().size() == 0 {
        Vec::new()
    } else {
        deserialize::<Vec<(String, u64)>>(&file.read())
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
}*/

fn run_perf_test<T: KernelPerf>(name: &str) {
    /*let mut test_struct = T::setup();

    set_print_color(TextColor::DarkGray, TextColor::Black);
    print!("Benchmarking");
    set_print_color(TextColor::LightCyan, TextColor::Black);
    print!(" {name}");

    let mut total_instr = 0;
    for _ in 0..PERF_TEST_ITERATIONS {
        let start_instr = get_instruction_count();
        test_struct.run();
        let end_instr = get_instruction_count();
        total_instr += end_instr - start_instr;
    }
    test_struct.teardown();

    if name.len() > 39 {
        panic!("Too long test name");
    }

    let num_spaces = 40 - name.len();
    for _ in 0..num_spaces {
        print!(" ");
    }

    let perf_instr = total_instr / PERF_TEST_ITERATIONS;

    let saved_perf_instr = get_perf_data(name);

    set_print_color(TextColor::White, TextColor::Black);
    print!("{}", perf_instr);
    set_print_color(TextColor::LightGray, TextColor::Black);
    print!(" instr");

    if let Some(saved_perf_ms) = saved_perf_instr {
        let percent = perf_instr as f32 / saved_perf_ms as f32 * 100.0 - 100.0;

        print!(" / ");
        set_print_color(TextColor::White, TextColor::Black);
        print!("{}", saved_perf_ms);
        set_print_color(TextColor::LightGray, TextColor::Black);
        print!(" instr");

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

    save_perf_data(name, perf_instr);*/
}
