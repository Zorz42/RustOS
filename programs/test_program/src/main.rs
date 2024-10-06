#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::panic::PanicInfo;

global_asm!(include_str!("asm/entry.S"));

#[no_mangle]
fn rust_entry() -> i32 {
    //main()

    for i in 10..20 {
        unsafe {
            asm!(r#"
            mv a7, {0}
            ecall
            "#, in(reg) i);
        }
    }

    loop {

    }
}

const ARRAY_SIZE: usize = 100000;
static mut ARRAY: [u32; ARRAY_SIZE] = [0; ARRAY_SIZE];

pub fn main() -> i32 {
    unsafe {
        ARRAY[0] = 0;
        ARRAY[1] = 1;
        for i in 2..ARRAY_SIZE {
            ARRAY[i] = ARRAY[i - 1].wrapping_add(ARRAY[i - 2]);
        }
        ARRAY[ARRAY_SIZE - 1] as i32
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
