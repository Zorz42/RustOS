#![no_std]
#![no_main]
#![feature(naked_functions)]
#![allow(non_snake_case)]
#![feature(raw_ref_op)]
#![feature(abi_x86_interrupt)]

use core::arch::asm;
use core::panic::PanicInfo;

use bootloader_api::config::Mapping;
use bootloader_api::info::PixelFormat;
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};

use crate::interrupts::init_idt;
use crate::memory::{check_page_table_integrity, init_memory, map_framebuffer, FRAMEBUFFER_OFFSET, KERNEL_STACK_ADDR, KERNEL_STACK_SIZE, VIRTUAL_OFFSET};
use crate::print::{reset_print_color, set_print_color, TextColor};
use crate::timer::init_timer;
use crate::vga_driver::clear_screen;

mod font;
mod interrupts;
mod memory;
mod ports;
mod print;
mod tests;
mod timer;
mod vga_driver;

const CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.kernel_stack_size = KERNEL_STACK_SIZE;
    config.mappings.physical_memory = Some(Mapping::FixedAddress(VIRTUAL_OFFSET));
    config.mappings.kernel_stack = Mapping::FixedAddress(KERNEL_STACK_ADDR);
    config.mappings.framebuffer = Mapping::FixedAddress(FRAMEBUFFER_OFFSET);
    config
};
entry_point!(kernel_main, config = &CONFIG);

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

    init_memory(&boot_info.memory_regions);

    // make sure that framebuffer ram has also occupied pages
    map_framebuffer(height as u32, stride as u32, bytes_per_pixel as u32);

    check_page_table_integrity();

    #[cfg(feature = "run_tests")]
    {
        use crate::tests::test_runner;
        test_runner();
    }

    println!("Going to infinite loop...");
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    set_print_color(TextColor::LightRed, TextColor::Black);
    println!("Kernel panic: {}", info);
    loop {}
}
