use crate::interrupts::{ExceptionStackFrame, set_idt_entry};
use crate::ports::{byte_in, byte_out};
use crate::println;

pub fn init_keyboard() {
    set_idt_entry(33, keyboard_handler);
}

extern "x86-interrupt" fn keyboard_handler(_stack_frame: &ExceptionStackFrame) {
    byte_out(0x20, 0x20);
    let code = byte_in(0x60);
    println!("Received keyboard signal {code}");
}