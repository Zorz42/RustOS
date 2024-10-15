#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::fmt;
use core::fmt::Write;
use core::panic::PanicInfo;
use std::{init_print, print, println};

global_asm!(include_str!("asm/entry.S"));

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

fn get_ticks() -> u64 {
    syscall0r(2)
}

fn get_pid() -> u64 {
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
pub fn _print(args: fmt::Arguments) {
    let mut writer = Writer;
    writer.write_fmt(args).unwrap();
}

pub fn exit() -> ! {
    syscall0(4);
    loop {}
}

#[no_mangle]
fn rust_entry() -> ! {
    init_print(&_print);

    main();

    exit();
}

const ARRAY_SIZE: usize = 100000;
static mut ARRAY: [u32; ARRAY_SIZE] = [0; ARRAY_SIZE];

pub fn main() {
    println!("Hello, world!");

    let mut curr_ticks = get_ticks() / 1000;
    loop {
        if get_ticks() / 1000 != curr_ticks {
            curr_ticks = get_ticks() / 1000;
            println!("Ticks {}: {}", get_pid(), get_ticks());
            if curr_ticks >= 5 + get_pid() {
                break;
            }
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("panic: {}", info);
    loop {

    }
}
