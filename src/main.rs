#![no_std]
#![no_main]

use core::panic::PanicInfo;
use std::println;
use crate::boot::infinite_loop;
use crate::memory::{get_num_free_pages, init_paging, init_paging_hart, NUM_PAGES};
use crate::print::{init_print, set_print_color, TextColor};
use crate::riscv::{get_core_id, get_mstatus, get_sstatus, interrupts_enable, set_mstatus, set_sstatus};
#[cfg(feature = "run_tests")]
use crate::tests::test_runner;
use crate::trap::init_trap;

mod boot;
mod riscv;
mod spinlock;
mod print;
mod timer;
mod trap;
mod memory;
#[cfg(feature = "run_tests")]
mod tests;

fn enable_fpu() {
    let mut sstatus = get_sstatus();
    sstatus |= 1 << 13;
    set_sstatus(sstatus);
}

pub fn main() {
    static mut INITIALIZED: bool = false;

    if get_core_id() == 0 {
        init_print();
        println!("Initializing kernel with core 0");
        init_trap();
        interrupts_enable(true);
        enable_fpu();
        init_paging();

        unsafe {
            INITIALIZED = true;
        }

    } else {
        while unsafe { !INITIALIZED } {}

        init_trap();
        interrupts_enable(true);
        enable_fpu();
        init_paging_hart();
    }

    println!("Core {} has initialized", get_core_id());

    if get_core_id() == 0 {
        #[cfg(feature = "run_tests")]
        test_runner();

        let all_memory = (NUM_PAGES * 4) as f32 / 1000.0;
        let used_memory = ((NUM_PAGES - get_num_free_pages()) * 4) as f32 / 1000.0;
        let portion = used_memory / all_memory * 100.0;
        println!("{used_memory} MB / {all_memory} MB of RAM used ({portion:.1}%)");
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    set_print_color(TextColor::LightRed, TextColor::Black);
    println!("Kernel panic: {}", info);
    infinite_loop();
}

