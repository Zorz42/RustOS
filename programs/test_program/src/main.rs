#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::fmt;
use core::panic::PanicInfo;
use std::{init_print, println};

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

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    for c in args.as_str().unwrap_or("").bytes() {
        print_char(c);
    }
}

#[no_mangle]
fn rust_entry() -> ! {
    init_print(&_print);

    main();

    loop {

    }
}

//const ARRAY_SIZE: usize = 100000;
//static mut ARRAY: [u32; ARRAY_SIZE] = [0; ARRAY_SIZE];

pub fn main() -> i32 {
    //let mut i = 0;
    loop {
        println!("Hello, World!");
        //i += 1;
    }

    /*unsafe {
        ARRAY[0] = 0;
        ARRAY[1] = 1;
        for i in 2..ARRAY_SIZE {
            ARRAY[i] = ARRAY[i - 1].wrapping_add(ARRAY[i - 2]);
        }
        ARRAY[ARRAY_SIZE - 1] as i32
    }*/
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
