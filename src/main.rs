#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(concat_idents)]

use core::arch::asm;
use core::panic::PanicInfo;
use crate::boot::infinite_loop;
use crate::print::{set_print_color, TextColor};
use crate::riscv::{get_core_id, get_sstatus, set_sstatus, SSTATUS_SIE};
use crate::trap::init_trap;

mod boot;
mod riscv;
mod spinlock;
mod print;
mod timer;
mod trap;

fn enable_interrupts() {
    let mut sstatus = get_sstatus();
    sstatus |= SSTATUS_SIE;
    set_sstatus(sstatus);
}

pub fn main() {
    init_trap();
    enable_interrupts();

    println!("Core {} has initialized", get_core_id());
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    set_print_color(TextColor::LightRed, TextColor::Black);
    println!("Kernel panic: {}", info);
    infinite_loop();
}

