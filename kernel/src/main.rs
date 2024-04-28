#![no_std]
#![no_main]
#![feature(naked_functions)]
#![allow(non_snake_case)]

use core::arch::asm;
use core::panic::PanicInfo;

use bootloader_api::config::Mapping;
use bootloader_api::info::PixelFormat;
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};

use crate::interrupts::init_idt;
use crate::memory::{init_memory, HEAP_BASE, HEAP_TREE, KERNEL_STACK_ADDR, KERNEL_STACK_SIZE, TESTING_OFFSET, VIRTUAL_OFFSET};
use crate::print::{reset_print_color, set_print_color, TextColor};
use crate::timer::init_timer;
use crate::vga_driver::clear_screen;

mod font;
mod interrupts;
mod memory;
mod ports;
mod print;
mod rand;
mod tests;
mod timer;
mod vga_driver;

const CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.kernel_stack_size = KERNEL_STACK_SIZE;
    config.mappings.physical_memory = Some(Mapping::FixedAddress(VIRTUAL_OFFSET));
    config.mappings.kernel_stack = Mapping::FixedAddress(KERNEL_STACK_ADDR);
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

    println!("Framebuffer is at    0x{:x}", framebuffer.as_mut_ptr() as u64);
    println!("Virtual offset is at 0x{:x}", VIRTUAL_OFFSET);
    println!("Kernel stack is at   0x{:x}", KERNEL_STACK_ADDR);
    println!("Heap base is at      0x{:x}", HEAP_BASE);
    println!("Heap tree is at      0x{:x}", HEAP_TREE);
    println!("Testing addr is at   0x{:x}", TESTING_OFFSET);

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

    #[cfg(feature = "run_tests")]
    {
        use crate::tests::test_runner;
        test_runner();
    }

    //let start = get_ticks();

    //for i in 0..100 {
    //println!("Iteration: {}", i);
    //}

    //println!("That took {}ms", get_ticks() - start);

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
