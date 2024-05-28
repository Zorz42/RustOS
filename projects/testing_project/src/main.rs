#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

static mut arr: [i32; 10000] = [0; 10000];

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> i32 {
    unsafe {
        arr[1] = 1;
        for i in 2..10000 {
            arr[i] = (arr[i - 1] + arr[i - 2]) % 1000;
        }
        //asm!("hlt");
        loop {}
    }
}

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
