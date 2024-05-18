#![no_std]
#![no_main]
#![feature(naked_functions)]
#![allow(non_snake_case)]
#![feature(raw_ref_op)]
#![feature(abi_x86_interrupt)]

use core::arch::asm;
use core::panic::PanicInfo;

use bootloader_api::{BootInfo, BootloaderConfig, entry_point};
use bootloader_api::config::Mapping;
use bootloader_api::info::PixelFormat;

use crate::disk::scan_for_disks;
use crate::filesystem::{close_fs, get_fs, init_fs};
use crate::interrupts::init_idt;
use crate::keyboard::init_keyboard;
use crate::memory::{check_page_table_integrity, FRAMEBUFFER_OFFSET, get_num_free_pages, get_num_pages, init_memory, KERNEL_STACK_ADDR, KERNEL_STACK_SIZE, map_framebuffer, map_page_auto, PAGE_SIZE, VirtAddr, VIRTUAL_OFFSET};
use crate::memory_disk::{get_mounted_disk, mount_disk, unmount_disk};
use crate::print::{reset_print_color, set_print_color, TextColor};
use crate::timer::init_timer;
use crate::vga_driver::clear_screen;

mod disk;
mod filesystem;
mod font;
mod interrupts;
mod memory;
mod memory_disk;
mod ports;
mod print;
#[cfg(feature = "run_tests")]
mod tests;
mod timer;
mod vga_driver;
mod keyboard;

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

    println!("Finding root disk");
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
    
    mount_disk(root_disk);

    // TEMPORARY
    get_mounted_disk().erase();
    
    init_fs();

    // TEMPORARY
    get_fs().erase();
    println!("Root disk is mounted!");

    #[cfg(feature = "run_tests")]
    {
        use crate::tests::test_runner;
        test_runner(&disks);
    }

    // run program
    /*let testing_program = include_bytes!("../../compiled_projects/testing_project");
    assert_eq!(testing_program[1] as char, 'E');
    assert_eq!(testing_program[2] as char, 'L');
    assert_eq!(testing_program[3] as char, 'F');

    let mut entry = 0x1000;
    for i in 0..8 {
        entry += (testing_program[24 + i] as u64) << (i * 8);
    }
    let program_offset = 1u64 << (12 + 3 * 9 + 2);

    println!("Mapping pages");
    let num_pages = (testing_program.len() as u64 + PAGE_SIZE - 1) / PAGE_SIZE;
    for i in 0..num_pages {
        map_page_auto((program_offset + PAGE_SIZE * i) as VirtAddr, true, true);
    }

    println!("Loading program");
    unsafe {
        memcpy_non_aligned(testing_program.as_ptr(), program_offset as *mut u8, testing_program.len());

        println!("Jumping into program (to 0x{entry:x})");
        asm!("call {}", in(reg) entry);

        let rax: u64;
        asm!("mov {}, rax", out(reg) rax);
        println!("Returned {rax}");
    }*/

    let all_memory = (get_num_pages() * 4) as f32 / 1000.0;
    let used_memory = ((get_num_pages() - get_num_free_pages()) * 4) as f32 / 1000.0;
    let portion = used_memory / all_memory * 100.0;
    println!("{used_memory} MB / {all_memory} MB used ({portion:.1}%)");
    
    close_fs();
    unmount_disk();
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
