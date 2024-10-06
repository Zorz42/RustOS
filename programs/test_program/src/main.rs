#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::panic::PanicInfo;

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

#[no_mangle]
fn rust_entry() -> ! {
    main();

    let string = "Hello, world!\n";
    for &c in string.as_bytes() {
        print_char(c);
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
