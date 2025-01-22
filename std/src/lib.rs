#![no_std]

mod syscall;

use core::fmt;
use core::fmt::Write;
use core::panic::PanicInfo;
use kernel_std::{init_print, init_std_memory};

pub use kernel_std::{print, println, Vec, String, Box, Mutable};
pub use std_derive::main as std_main;
use crate::syscall::{syscall0, syscall0r, syscall1, syscall2, SyscallCode};

extern "C" {
    fn main();
}

#[doc(hidden)]
pub fn _on_panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    exit();
}

fn alloc_page(addr: *mut u8, ignore_if_exists: bool) {
    syscall2(SyscallCode::AllocPage, addr as u64, ignore_if_exists as u64);
}

fn dealloc_page(addr: *mut u8) {
    syscall1(SyscallCode::DeallocPage, addr as u64);
}

#[doc(hidden)]
pub fn _init() -> ! {
    let ram_start = 1u64 << 34;
    let frame_size = 1u64 << 30;
    init_std_memory(&alloc_page, &dealloc_page, ram_start + frame_size, ram_start + 2 * frame_size, ram_start + frame_size);

    init_print(&_print);

    unsafe {
        main();
    }

    exit();
}

fn print_str(s: &str) {
    syscall2(SyscallCode::PrintStr, s.as_ptr() as u64, s.len() as u64);
}

pub fn get_ticks() -> u64 {
    syscall0r(SyscallCode::GetTicks)
}

pub fn get_pid() -> u64 {
    syscall0r(SyscallCode::GetPid)
}

struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        print_str(s);
        Ok(())
    }
}

#[doc(hidden)]
fn _print(args: fmt::Arguments) {
    let mut writer = Writer;
    writer.write_fmt(args).unwrap();
}

pub fn exit() -> ! {
    syscall0(SyscallCode::Exit);
    loop {}
}


