#![no_std]
#![no_main]

use crate::boot::infinite_loop;
use crate::disk::disk::{Disk, scan_for_disks};
use crate::memory::{get_num_free_pages, init_paging, init_paging_hart, KERNEL_OFFSET, NUM_PAGES};
use crate::print::{init_print, reset_print_color, set_print_color, TextColor};
use crate::riscv::{enable_fpu, get_core_id, interrupts_enable};
use crate::trap::init_trap;
use core::panic::PanicInfo;
use core::sync::atomic::{fence, Ordering};
use std::{print, println, Vec};
use crate::disk::filesystem::{close_fs, init_fs};
use crate::disk::memory_disk::mount_disk;
use crate::gpu::init_gpu;
use crate::keyboard::init_keyboard;
use crate::plic::{plicinit, plicinithart};

mod boot;
mod disk;
mod memory;
mod print;
mod riscv;
mod spinlock;
#[cfg(feature = "run_tests")]
mod tests;
mod timer;
mod trap;
mod virtio;
mod plic;
mod gpu;
mod font;
mod keyboard;

pub const ROOT_MAGIC: u32 = 0x63726591;

fn find_root_disk(disks: &mut Vec<Disk>) -> Disk {
    for disk in disks {
        let first_sector = disk.read(0);

        let mut magic = 0;

        for i in 0..4 {
            magic += (first_sector[511 - i] as u32) << (8 * i);
        }

        if ROOT_MAGIC == magic {
            return disk.clone();
        }
    }
    panic!("Root disk not found")
}

pub fn main() {
    static mut INITIALIZED: bool = false;

    if get_core_id() == 0 {
        init_trap();
        interrupts_enable(true);
        enable_fpu();
        init_paging();
        plicinit();
        plicinithart();

        init_print();
        init_gpu();
        let mut disks = scan_for_disks();

        println!("Initializing kernel with core 0");

        init_keyboard();

        #[cfg(debug_assertions)]
        {
            set_print_color(TextColor::LightGreen, TextColor::Black);
            println!("Debug mode enabled (this message should not be present in release builds)");
            reset_print_color();
        }

        #[cfg(feature = "run_tests")]
        {
            use crate::tests::test_runner;
            test_runner(&mut disks);
        }

        let root_disk = find_root_disk(&mut disks);

        close_fs();
        mount_disk(&root_disk);
        init_fs();

        #[cfg(feature = "run_perf")]
        {
            use crate::tests::perf_test_runner;
            perf_test_runner();
        }

        fence(Ordering::Release);
        unsafe {
            INITIALIZED = true;
        }
        fence(Ordering::Release);

        println!("Kernel initialized!");

        let all_memory = (NUM_PAGES * 4) as f32 / 1000.0;
        let used_memory = ((NUM_PAGES - get_num_free_pages()) * 4) as f32 / 1000.0;
        let portion = used_memory / all_memory * 100.0;
        println!("{used_memory} MB / {all_memory} MB of RAM used ({portion:.1}%)");
    } else {
        while unsafe { !INITIALIZED } {}

        init_trap();
        interrupts_enable(true);
        enable_fpu();
        init_paging_hart();
        plicinithart();
        println!("Core {} has initialized", get_core_id());
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    set_print_color(TextColor::LightRed, TextColor::Black);
    println!("Kernel panic: {}", info);
    infinite_loop();
}
