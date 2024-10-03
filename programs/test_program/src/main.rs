#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

global_asm!(include_str!("asm/entry.S"));

#[no_mangle]
fn rust_entry() -> i32 {
    return 42;
}

pub fn main() -> i32 {
    let mut a = 0;
    let mut b = 1;
    // iterate fibonacci 100 times
    for _ in 0..100 {
        let c = a + b;
        a = b;
        b = c;
    }
    a
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {}
}
