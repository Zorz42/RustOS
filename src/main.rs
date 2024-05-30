#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(concat_idents)]

use core::arch::global_asm;
global_asm!(include_str!("entry.S"));

mod boot;
//mod riscv;

#[no_mangle]
extern "C" fn eh_personality() {}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

