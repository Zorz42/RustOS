#![no_std]
use core::arch::asm;
use core::fmt;
use core::fmt::Write;
use core::panic::PanicInfo;
use kernel_std::init_print;

pub use kernel_std::{print, println};

extern "C" {
    fn main();
}

#[doc(hidden)]
pub fn _on_panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    exit();
}

#[doc(hidden)]
pub fn _init() -> ! {
    init_print(&_print);

    unsafe {
        main();
    }

    exit();
}

fn syscall0(code: u64) {
    unsafe {
        asm!("ecall", in("a7") code);
    }
}

fn syscall1(code: u64, arg1: u64) {
    unsafe {
        asm!("ecall", in("a7") code, in("a3") arg1);
    }
}

fn syscall2(code: u64, arg1: u64, arg2: u64) {
    unsafe {
        asm!("ecall", in("a7") code, in("a3") arg1, in("a4") arg2);
    }
}

fn syscall0r(code: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!("ecall", in("a7") code, out("a2") ret);
    }
    ret
}

fn print_str(s: &str) {
    syscall2(1, s.as_ptr() as u64, s.len() as u64);
}

pub fn get_ticks() -> u64 {
    syscall0r(2)
}

pub fn get_pid() -> u64 {
    syscall0r(3)
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
    syscall0(4);
    loop {}
}
