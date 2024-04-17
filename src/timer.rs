use crate::{interrupt_wrapper, println};
use core::arch::asm;
use crate::interrupts::set_idt_entry;
use crate::ports::byte_out;

pub fn init_timer() {
    set_idt_entry(32,interrupt_wrapper!(timer_handler));
    
    let frequency = 1000;
    let divisor = 1193180 / frequency;
    
    byte_out(0x43, 0x36);
    byte_out(0x40, (divisor & 0xFF) as u8);
    byte_out(0x40, ((divisor >> 8) & 0xFF) as u8);
}

extern "C" fn timer_handler() {
    byte_out(0x20, 0x20);
    
    println!("Timer interrupt");
}