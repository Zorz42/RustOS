#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(concat_idents)]

use core::panic::PanicInfo;
use crate::boot::infinite_loop;
use crate::memory::{_end, parse_dtb};
use crate::print::{set_print_color, TextColor};
use crate::riscv::{get_core_id, get_sstatus, interrupts_enable, set_sstatus, SSTATUS_SIE};
use crate::timer::get_ticks;
use crate::trap::init_trap;

mod boot;
mod riscv;
mod spinlock;
mod print;
mod timer;
mod trap;
mod memory;

pub fn main() {
    init_trap();
    interrupts_enable(true);

    const DTB_ADDRESS: usize = 0x80000000; // Replace with the actual DTB address

    let dtb_ptr = DTB_ADDRESS as *const u8;
    let ram_size = parse_dtb(dtb_ptr);

    println!("Total RAM size: {} bytes", ram_size);
    
    println!("Core {} has initialized", get_core_id());

    if get_core_id() == 0 {
        let mut ticker = get_ticks();
        let mut count = 0;
        while count < 10 {
            println!("Count {count}");
            while get_ticks() - ticker < 1000 {}
            ticker = get_ticks();
            count += 1;
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    set_print_color(TextColor::LightRed, TextColor::Black);
    println!("Kernel panic: {}", info);
    infinite_loop();
}

