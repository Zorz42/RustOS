use crate::interrupts::{set_idt_entry, ExceptionStackFrame};
use crate::ports::byte_out;

pub fn init_timer() {
    set_idt_entry(32, timer_handler);

    let frequency = 1000;
    let divisor = 1193180 / frequency;

    byte_out(0x43, 0x36);
    byte_out(0x40, (divisor & 0xFF) as u8);
    byte_out(0x40, ((divisor >> 8) & 0xFF) as u8);
}

pub static mut TIMER_TICKS: u32 = 0;

extern "x86-interrupt" fn timer_handler(_stack_frame: &ExceptionStackFrame) {
    byte_out(0x20, 0x20);

    unsafe {
        TIMER_TICKS += 1;
    }
}

pub fn get_ticks() -> u32 {
    unsafe { TIMER_TICKS }
}
