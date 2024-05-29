#![no_std]
#![no_main]
#![feature(naked_functions)]

mod boot;

#[no_mangle]
extern "C" fn eh_personality() {}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

