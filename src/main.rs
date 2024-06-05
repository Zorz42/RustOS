#![no_std]
#![no_main]

use crate::boot::infinite_loop;
use crate::disk::disk::scan_for_disks;
use crate::memory::{get_num_free_pages, init_paging, init_paging_hart, NUM_PAGES};
use crate::print::{init_print, set_print_color, TextColor};
use crate::riscv::{enable_fpu, get_core_id, interrupts_enable};
#[cfg(feature = "run_tests")]
use crate::tests::test_runner;
use crate::trap::init_trap;
use core::panic::PanicInfo;
use core::sync::atomic::{fence, Ordering};
use std::println;
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

pub fn main() {
    static mut INITIALIZED: bool = false;

    if get_core_id() == 0 {
        init_print();
        println!("Initializing kernel with core 0");
        init_trap();
        interrupts_enable(true);
        enable_fpu();
        init_paging();
        let mut disks = scan_for_disks();
        plicinit();
        plicinithart();

        //let disk = &mut disks[0];
        //disk.read(2);

        fence(Ordering::Release);
        unsafe {
            INITIALIZED = true;
        }
        fence(Ordering::Release);

        println!("Kernel initialized!");

        #[cfg(feature = "run_tests")]
        test_runner(&mut disks);

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
