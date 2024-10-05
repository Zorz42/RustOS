#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::panic::PanicInfo;

global_asm!(include_str!("asm/entry.S"));

#[no_mangle]
fn rust_entry() -> i32 {
    //main()
    unsafe {
        asm!(r#"
        li a7, 42
        ecall
        "#);
    }

    loop {

    }
}

const ARRAY_SIZE: usize = 100000;
static mut array: [u32; ARRAY_SIZE] = [0; ARRAY_SIZE];

pub fn main() -> i32 {
    unsafe {
        array[0] = 0;
        array[1] = 1;
        for i in 2..ARRAY_SIZE {
            array[i] = array[i - 1].wrapping_add(array[i - 2]);
        }
        array[ARRAY_SIZE - 1] as i32
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
