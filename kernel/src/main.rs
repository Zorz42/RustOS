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
mod font;

use core::arch::asm;
use core::panic::PanicInfo;
use crate::print::{reset_print_color, set_print_color, TextColor};
use bootloader_api::{entry_point, BootInfo};
use crate::vga_driver::clear_screen;

const CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.kernel_stack_size = 100 * 1024; // 100 KiB
    config
};
entry_point!(kernel_main, config = &CONFIG);

// this is only so that ide doesn't complain about non-existence of test_main
// it should be excluded from compilation when in test mode
#[cfg(not(test))]
fn test_main(){}

fn init_display(boot_info: &'static mut BootInfo) {
    let binding = boot_info.framebuffer.as_mut().unwrap();
    assert_eq!(binding.info().pixel_format, bootloader_api::info::PixelFormat::Bgr);
    let width = binding.info().width;
    let height = binding.info().height;
    let stride = binding.info().stride;
    let bytes_per_pixel = binding.info().bytes_per_pixel;
    let framebuffer = binding.buffer_mut();

    vga_driver::init(width, height, stride, bytes_per_pixel, framebuffer.as_mut_ptr());
}

#[no_mangle]
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    init_display(boot_info);
    clear_screen();
    
    println!("Booting kernel...");

    #[cfg(debug_assertions)]
    {
        set_print_color(TextColor::LightGreen, TextColor::Black);
        println!("Debug mode enabled (this message should not be present in release builds)");
        reset_print_color();
    }
    
    interrupts::init_idt();
    //timer::init_timer();

    #[cfg(test)]
    test_main();
    
    unsafe {
        //asm!("int 0x03");
    }
    
    println!("Going to infinite loop...");
    let mut i = 0;
    loop {
        /*unsafe {
            asm!("hlt");
        }*/
        //print!("Ticks: {}\r", timer::get_ticks());
        
        i += 1;
        println!("Iteration: {}", i);
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    set_print_color(TextColor::LightRed, TextColor::Black);
    println!("Kernel panic: {}", info);
    loop {}
}