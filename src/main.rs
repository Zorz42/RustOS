#![no_std]
#![no_main]

use crate::boot::infinite_loop;
use crate::disk::disk::{Disk, scan_for_disks};
use crate::memory::{get_num_free_pages, init_paging, init_paging_hart, NUM_PAGES};
use crate::print::{init_print, reset_print_color, set_print_color, TextColor};
use crate::riscv::{enable_fpu, get_core_id, interrupts_enable};
use crate::trap::init_trap;
use core::panic::PanicInfo;
use core::sync::atomic::{fence, Ordering};
use std::{println, Vec};
use crate::disk::filesystem::{close_fs, init_fs};
use crate::disk::memory_disk::mount_disk;
use crate::gpu::init_gpu;
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

fn find_root_disk(disks: &mut Vec<&'static mut Disk>) -> &'static mut Disk {
    let mut root_disk = None;
    for disk in disks {
        let first_sector = disk.read(0);
        let root_magic = 0x63726591;

        let mut magic = 0;

        for i in 0..4 {
            magic += (first_sector[511 - i] as u32) << (8 * i);
        }

        if root_magic == magic {
            root_disk = Some(*disk as *mut Disk);
        }
    }
    unsafe { &mut **root_disk.as_mut().unwrap() }
}

pub fn main() {
    static mut INITIALIZED: bool = false;

    if get_core_id() == 0 {
        init_print();
        println!("Initializing kernel with core 0");
        #[cfg(debug_assertions)]
        {
            set_print_color(TextColor::LightGreen, TextColor::Black);
            println!("Debug mode enabled (this message should not be present in release builds)");
            reset_print_color();
        }

        init_trap();
        interrupts_enable(true);
        enable_fpu();
        init_paging();
        plicinit();
        plicinithart();

        let mut disks = scan_for_disks();
        init_gpu();

        #[cfg(feature = "run_tests")]
        {
            use crate::tests::test_runner;
            test_runner(&mut disks);
        }

        let root_disk = find_root_disk(&mut disks);

        close_fs();
        mount_disk(root_disk);
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
