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
mod memory;

use core::arch::asm;
use core::panic::PanicInfo;
use crate::print::{reset_print_color, set_print_color, TextColor};
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use bootloader_api::config::Mapping;
use bootloader_api::info::PixelFormat;
use crate::interrupts::init_idt;
use crate::memory::{init_memory, KERNEL_STACK_ADDR, KERNEL_STACK_SIZE, VIRTUAL_OFFSET};
use crate::timer::{get_ticks, init_timer};
use crate::vga_driver::clear_screen;

const CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.kernel_stack_size = KERNEL_STACK_SIZE;
    config.mappings.physical_memory = Some(Mapping::FixedAddress(VIRTUAL_OFFSET));
    config.mappings.kernel_stack = Mapping::FixedAddress(KERNEL_STACK_ADDR);
    config
};
entry_point!(kernel_main, config = &CONFIG);

// this is only so that ide doesn't complain about non-existence of test_main
// it should be excluded from compilation when in test mode
#[cfg(not(test))]
fn test_main(){}

#[no_mangle]
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let binding = boot_info.framebuffer.as_mut().unwrap();
    assert_eq!(binding.info().pixel_format, PixelFormat::Bgr);
    let width = binding.info().width;
    let height = binding.info().height;
    let stride = binding.info().stride;
    let bytes_per_pixel = binding.info().bytes_per_pixel;
    let framebuffer = binding.buffer_mut();

    vga_driver::init(width, height, stride, bytes_per_pixel, framebuffer.as_mut_ptr());
    
    clear_screen();

    println!("Booting kernel...");

    debug_assert!(boot_info.physical_memory_offset.take().is_some());
    
    #[cfg(debug_assertions)]
    {
        set_print_color(TextColor::LightGreen, TextColor::Black);
        println!("Debug mode enabled (this message should not be present in release builds)");
        reset_print_color();
    }
    
    init_idt();
    init_timer();
    
    init_memory(&boot_info.memory_regions, framebuffer.as_ptr() as u64, binding.info().byte_len as u64, boot_info.kernel_addr, boot_info.kernel_image_offset, boot_info.kernel_len);

    #[cfg(test)]
    test_main();
    
    unsafe {
        //asm!("int 0x03");
    }
    
    let start = get_ticks();
    
    for i in 0..100 {
        println!("Iteration: {}", i);
    }
    
    println!("That took {}ms", get_ticks() - start);
    
    println!("Going to infinite loop...");
    //let mut i = 0;
    loop {
        unsafe {
            asm!("hlt");
        }
        //print!("Ticks: {}\r", get_ticks());
        
        //i += 1;
        //println!("Iteration: {}", i);
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    set_print_color(TextColor::LightRed, TextColor::Black);
    println!("Kernel panic: {}", info);
    loop {}
}