#![no_std]
#![no_main]
#![feature(core_intrinsics)]

mod vga_driver;
mod print;

use core::panic::PanicInfo;
use crate::print::{reset_print_color, set_print_color};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Booting kernel...");

    #[cfg(debug_assertions)]
    {
        set_print_color(vga_driver::VgaColor::LightGreen, vga_driver::VgaColor::Black);
        println!("Debug mode enabled (this message should not be present in release builds)");
        reset_print_color();
    }
    
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    set_print_color(vga_driver::VgaColor::LightRed, vga_driver::VgaColor::Black);
    println!("Kernel panic: {}", info);
    loop {}
}