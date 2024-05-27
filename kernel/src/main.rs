#![no_std]
#![no_main]
#![feature(naked_functions)]
#![allow(non_snake_case)]
#![feature(raw_ref_op)]
#![feature(abi_x86_interrupt)]

use core::arch::asm;
use core::cmp::PartialEq;
use core::panic::PanicInfo;

use bootloader_api::config::Mapping;
use bootloader_api::info::PixelFormat;
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use std::{String, Vec};

use crate::disk::disk::scan_for_disks;
use crate::disk::filesystem::{close_fs, get_fs, init_fs};
use crate::interrupts::init_idt;
use crate::keyboard::{init_keyboard};
use crate::memory::{
    check_page_table_integrity, get_num_free_pages, get_num_pages, init_memory, map_framebuffer, FRAMEBUFFER_OFFSET, KERNEL_STACK_ADDR, KERNEL_STACK_SIZE,
    VIRTUAL_OFFSET,
};
use crate::disk::memory_disk::{get_mounted_disk, mount_disk, unmount_disk};
use crate::ports::word_out;
use crate::print::{reset_print_color, set_print_color, TextColor};
use crate::shell::shell_main;
use crate::timer::init_timer;
use crate::vga_driver::clear_screen;

mod disk;
mod font;
mod interrupts;
mod keyboard;
mod memory;
mod ports;
mod print;
#[cfg(feature = "run_tests")]
mod tests;
mod timer;
mod vga_driver;
mod shell;

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

    println!("Initializing IDT");
    init_idt();
    init_timer();
    init_keyboard();

    println!("Initializing memory");
    init_memory(&boot_info.memory_regions);
    // make sure that framebuffer ram has also occupied pages
    map_framebuffer(height as u32, stride as u32, bytes_per_pixel as u32);
    // make sure that the page table setup by bootloader has a few properties that we want
    check_page_table_integrity();

    println!("Initializing disk");
    let disks = scan_for_disks();

    #[cfg(feature = "run_tests")]
    {
        use crate::tests::test_runner;
        test_runner(&disks);
    }

    // find the root disk
    let mut root_disk = None;
    for disk in &disks {
        let first_sector = disk.read(0);
        let root_magic = 0x63726591;

        let mut magic = 0;

        for i in 0..4 {
            magic += (first_sector[511 - i] as u32) << (8 * i);
        }

        if root_magic == magic {
            root_disk = Some(disk.clone());
        }
    }
    let root_disk = root_disk.unwrap();
    
    close_fs();
    mount_disk(root_disk.clone());
    init_fs();
    
    println!("Root disk is mounted!");

    let program_exec = include_bytes!("../../compiled_projects/testing_project");
    let file = get_fs().create_file(&String::from("programs/test"));
    file.write(&Vec::new_from_slice(program_exec));
    
    #[cfg(feature = "run_perf")]
    {
        use crate::tests::perf_test_runner;
        perf_test_runner();
    }

    let all_memory = (get_num_pages() * 4) as f32 / 1000.0;
    let used_memory = ((get_num_pages() - get_num_free_pages()) * 4) as f32 / 1000.0;
    let portion = used_memory / all_memory * 100.0;
    println!("{used_memory} MB / {all_memory} MB of RAM used ({portion:.1}%)");

    let all_disk = (get_mounted_disk().get_num_pages() * 4) as f32 / 1000.0;
    let used_disk = ((get_mounted_disk().get_num_pages() - get_mounted_disk().get_num_free_pages()) * 4) as f32 / 1000.0;
    let portion = used_disk / all_disk * 100.0;
    println!("{used_disk} MB / {all_disk} MB of DISK used ({portion:.1}%)");
    
    println!("Entering shell");
    
    shell_main();

    close_fs();
    unmount_disk();
    mount_disk(root_disk.clone());
    unmount_disk();

    // shutdown qemu
    word_out(0xB004, 0x2000);
    word_out(0x604, 0x2000);
    
    println!("Going to infinite loop, because QEMU did not shut down");
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
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
