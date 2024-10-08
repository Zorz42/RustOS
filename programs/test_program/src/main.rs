#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::fmt;
use core::fmt::Write;
use core::panic::PanicInfo;
use std::{init_print, print, println};

global_asm!(include_str!("asm/entry.S"));

fn print_char(c: u8) {
    unsafe {
        asm!(r#"
        li a2, 1
        mv a3, {0}
        ecall
        "#, in(reg) c as u64);
    }
}

struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            print_char(c);
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    let mut writer = Writer;
    writer.write_fmt(args).unwrap();
}

#[no_mangle]
fn rust_entry() -> ! {
    init_print(&_print);

    main();

    loop {

    }
}

const ARRAY_SIZE: usize = 100000;
static mut ARRAY: [u32; ARRAY_SIZE] = [0; ARRAY_SIZE];

pub fn main() -> i32 {
    println!("Hello, world!");

    loop {

    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("panic!");
    loop {

    }
}
