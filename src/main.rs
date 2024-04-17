#![no_std]
#![no_main]
#![feature(core_intrinsics)]
#![feature(custom_test_frameworks)]
#![feature(naked_functions)]
#![test_runner(crate::tests::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod vga_driver;
mod print;
mod tests;
mod interrupts;
mod ports;
mod timer;

use core::arch::asm;
use core::panic::PanicInfo;
use crate::print::{reset_print_color, set_print_color};

// this is only so that ide doesn't complain about non-existence of test_main
// it should be excluded from compilation when in test mode
#[cfg(not(test))]
fn test_main(){}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Booting kernel...");

    #[cfg(debug_assertions)]
    {
        set_print_color(vga_driver::VgaColor::LightGreen, vga_driver::VgaColor::Black);
        println!("Debug mode enabled (this message should not be present in release builds)");
        reset_print_color();
    }
    
    interrupts::init_idt();
    timer::init_timer();

    #[cfg(test)]
    test_main();
    
    unsafe {
        asm!("int 0x03");
    }
    
    println!("Going to infinite loop...");
    loop {
        unsafe {
            asm!("hlt");
        }
        println!("Iterating loop");
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    set_print_color(vga_driver::VgaColor::LightRed, vga_driver::VgaColor::Black);
    println!("Kernel panic: {}", info);
    loop {}
}