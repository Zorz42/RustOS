#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(concat_idents)]

use core::panic::PanicInfo;
use crate::boot::infinite_loop;
use crate::memory::{get_num_free_pages, init_paging, NUM_PAGES};
use crate::print::{set_print_color, TextColor};
use crate::riscv::{get_core_id, interrupts_enable};
use crate::trap::init_trap;

mod boot;
mod riscv;
mod spinlock;
mod print;
mod timer;
mod trap;
mod memory;

pub fn main() {
    if get_core_id() == 0 {
        init_trap();
        interrupts_enable(true);
        init_paging();

        let all_memory = (NUM_PAGES * 4) as f32 / 1000.0;
        let used_memory = ((NUM_PAGES - get_num_free_pages()) * 4) as f32 / 1000.0;
        let portion = used_memory / all_memory * 100.0;
        println!("{used_memory} MB / {all_memory} MB of RAM used ({portion:.1}%)");
    } else {
        init_trap();
        interrupts_enable(true);
    }

    println!("Core {} has initialized", get_core_id());
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    set_print_color(TextColor::LightRed, TextColor::Black);
    println!("Kernel panic: {}", info);
    infinite_loop();
}

