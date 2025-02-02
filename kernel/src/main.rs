#![no_std]
#![no_main]
#![allow(non_camel_case_types)]

use crate::boot::infinite_loop;
use crate::disk::disk::{Disk, scan_for_disks};
use crate::memory::{get_num_free_pages, init_paging, init_paging_hart, KERNEL_VIRTUAL_END, NUM_PAGES};
use crate::print::{init_print, set_print_color};
use crate::riscv::{enable_fpu, get_core_id, interrupts_enable};
use crate::trap::switch_to_kernel_trap;
use core::panic::PanicInfo;
use core::sync::atomic::{fence, Ordering};
use kernel_std::{debug_str, debugln, println, String, Vec};
use crate::console::run_console;
use crate::disk::filesystem::write_to_file;
use crate::disk::memory_disk::{mount_disk, unmount_disk};
use crate::gpu::init_gpu;
use crate::input::{init_input_devices};
use crate::plic::{plicinit, plicinithart};
use crate::scheduler::{scheduler, run_program, toggle_scheduler, init_scheduler};
use crate::text_renderer::{init_text_renderer, TextColor};

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
mod input;
mod console;
mod scheduler;
mod text_renderer;
mod elf;

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
        switch_to_kernel_trap();
        interrupts_enable(true);
        enable_fpu();
        init_paging();
        plicinit();
        plicinithart();

        init_gpu();
        init_print();
        init_text_renderer();
        let mut disks = scan_for_disks();

        println!("Initializing kernel with core 0");

        init_scheduler();
        init_input_devices();

        #[cfg(debug_assertions)]
        {
            set_print_color(TextColor::LightGreen, TextColor::Black);
            println!("Debug mode enabled (this message should not be present in release builds)");
            print::reset_print_color();
        }

        fence(Ordering::Release);
        unsafe {
            INITIALIZED = true;
        }
        fence(Ordering::Release);

        #[cfg(feature = "run_tests")]
        {
            use crate::tests::test_runner;
            test_runner(&mut disks);
        }

        let root_disk = find_root_disk(&mut disks);

        mount_disk(&root_disk);
        //fs_erase();

        #[cfg(feature = "run_perf")]
        {
            toggle_scheduler(false);
            use crate::tests::perf_test_runner;
            perf_test_runner();
            toggle_scheduler(true);
        }

        let all_memory = (NUM_PAGES * 4) as f32 / 1000.0;
        let used_memory = ((NUM_PAGES - get_num_free_pages()) * 4) as f32 / 1000.0;
        let portion = used_memory / all_memory * 100.0;
        println!("{used_memory} MB / {all_memory} MB of RAM used ({portion:.1}%)");

        run_console();

        unmount_disk();
        mount_disk(&root_disk);

    } else {
        while unsafe { !INITIALIZED } {
            fence(Ordering::Acquire);
        }

        switch_to_kernel_trap();
        interrupts_enable(true);
        enable_fpu();
        init_paging_hart();
        plicinithart();
        println!("Core {} has initialized", get_core_id());
    }

    scheduler();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    debugln!("Kernel panic: {}", info);
    set_print_color(TextColor::LightRed, TextColor::Black);
    println!("Kernel panic: {}", info);
    infinite_loop();
}
