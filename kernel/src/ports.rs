use core::arch::asm;

pub fn byte_in(port: u16) -> u8 {
    let result: u8;
    unsafe {
        asm!("in al, dx", in("dx") port, out("al") result);
    }
    result
}

pub fn byte_out(port: u16, data: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") data);
    }
}

pub fn word_in(port: u16) -> u16 {
    let result: u16;
    unsafe {
        asm!("in ax, dx", in("dx") port, out("ax") result);
    }
    result
}

pub fn word_out(port: u16, data: u16) {
    unsafe {
        asm!("out dx, ax", in("dx") port, in("ax") data);
    }
}