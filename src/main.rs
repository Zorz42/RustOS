#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(concat_idents)]

mod boot;
mod riscv;
mod graphics;

pub fn main() {

}

#[no_mangle]
extern "C" fn eh_personality() {}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

