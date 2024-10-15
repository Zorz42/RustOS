#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;
use std::{exit, get_pid, get_ticks, println};

global_asm!(r#"
.section .init

_start:
    j rust_entry

"#);

#[no_mangle]
extern "C" fn rust_entry() -> ! {
    std::init();
}

#[no_mangle]
pub extern "C" fn main() {
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
    exit();
}