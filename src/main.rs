#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(concat_idents)]

use crate::riscv::get_core_id;

mod boot;
mod riscv;
mod spinlock;
mod print;

pub fn main() {
    println!("Core {} entered main function...", get_core_id());
}

#[no_mangle]
extern "C" fn eh_personality() {}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

