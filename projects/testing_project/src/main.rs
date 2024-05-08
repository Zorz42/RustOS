#![no_std]
#![no_main]

use core::panic::PanicInfo;

static mut arr: [i32; 1024] = [0; 1024];

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> i32 {
    unsafe {
        arr[1] = 1;
        for i in 2..1024 {
            arr[i] = (arr[i - 1] + arr[i - 2]) % 1000;
        }
        arr[1023]
    }
}

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
