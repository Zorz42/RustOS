#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

global_asm!(include_str!("asm/entry.S"));

#[no_mangle]
fn rust_entry() -> i32 {
    main()
}

pub fn main() -> i32 {
    let mut a = 0u32;
    let mut b = 1u32;
    // iterate fibonacci 100 times
    for _ in 0..100 {
        let c = a.wrapping_add(b);
        a = b;
        b = c;
    }
    a as i32
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {}
}
