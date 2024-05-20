use core::arch::asm;
use core::ptr::addr_of;

use crate::disk::memory_disk::disk_page_fault_handler;
use crate::ports::byte_out;
use crate::println;

#[derive(Debug)]
#[repr(C)]
pub struct ExceptionStackFrame {
    instruction_pointer: u64,
    code_segment: u64,
    cpu_flags: u64,
    stack_pointer: u64,
    stack_segment: u64,
}

macro_rules! interrupt_message {
    ($name: expr) => {{
        extern "x86-interrupt" fn wrapper(_stack_frame: &ExceptionStackFrame) {
            println!("{} exception", $name);
            loop {}
        }
        wrapper
    }};
}

macro_rules! interrupt_message_ec {
    ($name: expr) => {{
        extern "x86-interrupt" fn wrapper(_stack_frame: &ExceptionStackFrame, error_code: u64) {
            println!("{} exception with error code {error_code} 0x{error_code:x} 0b{error_code:b}", $name);
            loop {}
        }
        wrapper
    }};
}

const IDT_ENTRY_SIZE: usize = 16;

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct IDTEntry {
    pointer_low: u16,
    selector: u16,
    options: u16,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32,
}

#[repr(C, packed)]
struct IDTPointer {
    limit: u16,
    base: u64,
}

const fn create_options(present: bool, enable_interrupts: bool) -> u16 {
    let mut options = 0;
    if present {
        options |= 1 << 15; // present
    }
    options |= 0b111 << 9; // must be one bits
    if enable_interrupts {
        options |= 1 << 8; // interrupt bit
    }
    options
}

impl IDTEntry {
    pub const fn new(pointer: u64, selector: u16, options: u16) -> IDTEntry {
        IDTEntry {
            pointer_low: pointer as u16,
            selector,
            options,
            pointer_middle: (pointer >> 16) as u16,
            pointer_high: (pointer >> 32) as u32,
            reserved: 0,
        }
    }
}

const IDT_SIZE: usize = 256;
static mut IDT: [IDTEntry; IDT_SIZE] = [IDTEntry::new(0, 0, create_options(false, true)); IDT_SIZE];

pub type HandlerFunc = extern "x86-interrupt" fn(&ExceptionStackFrame);

pub fn set_idt_entry(index: usize, handler: HandlerFunc) {
    unsafe {
        IDT[index] = IDTEntry::new((handler as usize) as u64, 0x08, create_options(true, false));
    }
}

pub type HandlerFuncEc = extern "x86-interrupt" fn(&ExceptionStackFrame, u64);

pub fn set_idt_entry_ec(index: usize, handler: HandlerFuncEc) {
    unsafe {
        IDT[index] = IDTEntry::new((handler as usize) as u64, 0x08, create_options(true, false));
    }
}

static mut IDT_POINTER: IDTPointer = IDTPointer {
    limit: (IDT_SIZE * IDT_ENTRY_SIZE - 1) as u16,
    base: 0,
};

pub fn init_idt() {
    unsafe {
        IDT_POINTER.base = addr_of!(IDT) as *const _ as u64;
    }

    set_idt_entry(0, interrupt_message!("Divide by zero"));
    set_idt_entry(1, interrupt_message!("Debug"));
    set_idt_entry(2, interrupt_message!("Non-maskable interrupt"));
    set_idt_entry(3, interrupt_message!("Breakpoint"));
    set_idt_entry(4, interrupt_message!("Overflow"));
    set_idt_entry(5, interrupt_message!("Bound range exceeded"));
    set_idt_entry(6, interrupt_message!("Invalid opcode"));
    set_idt_entry(7, interrupt_message!("Device not available"));
    set_idt_entry_ec(8, interrupt_message_ec!("Double fault"));
    set_idt_entry(9, interrupt_message!("Coprocessor segment overrun"));
    set_idt_entry_ec(10, interrupt_message_ec!("Invalid TSS"));
    set_idt_entry_ec(11, interrupt_message_ec!("Segment not present"));
    set_idt_entry_ec(12, interrupt_message_ec!("Stack-segment fault"));
    set_idt_entry_ec(13, interrupt_message_ec!("General protection fault"));
    set_idt_entry(15, interrupt_message!("Reserved"));
    set_idt_entry(16, interrupt_message!("x87 FPU floating-point error"));
    set_idt_entry_ec(17, interrupt_message_ec!("Alignment check"));
    set_idt_entry(18, interrupt_message!("Machine check"));
    set_idt_entry(19, interrupt_message!("SIMD floating-point"));
    set_idt_entry(20, interrupt_message!("Virtualization"));
    set_idt_entry_ec(21, interrupt_message_ec!("Control"));
    set_idt_entry(22, interrupt_message!("Reserved"));
    set_idt_entry(23, interrupt_message!("Reserved"));
    set_idt_entry(24, interrupt_message!("Reserved"));
    set_idt_entry(25, interrupt_message!("Reserved"));
    set_idt_entry(26, interrupt_message!("Reserved"));
    set_idt_entry(27, interrupt_message!("Reserved"));
    set_idt_entry(28, interrupt_message!("Reserved"));
    set_idt_entry(29, interrupt_message!("Reserved"));
    set_idt_entry(30, interrupt_message!("Security"));
    set_idt_entry(31, interrupt_message!("Reserved"));

    set_idt_entry_ec(14, page_fault_handler);

    // remap irq table to 0x20-0x2F
    // master PIC
    byte_out(0x20, 0x11);
    byte_out(0x21, 0x20);
    byte_out(0x21, 0x04);
    byte_out(0x21, 0x01);
    byte_out(0x21, 0x00);
    // slave PIC
    byte_out(0xA0, 0x11);
    byte_out(0xA1, 0x28);
    byte_out(0xA1, 0x02);
    byte_out(0xA1, 0x01);
    byte_out(0xA1, 0x00);

    unsafe {
        asm!("lidt [{}]", in(reg) addr_of!(IDT_POINTER));
        asm!("sti");
    }
}

extern "x86-interrupt" fn page_fault_handler(_stack_frame: &ExceptionStackFrame, error_code: u64) {
    let cr2: u64;
    unsafe {
        asm!("mov {}, cr2", out(reg) cr2);
    }

    if disk_page_fault_handler(cr2) {
        return;
    }

    println!("Page fault exception with error code {error_code} 0x{error_code:x} 0b{error_code:b} at address 0x{cr2:x}");
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
