use core::arch::asm;
use crate::println;

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

pub type HandlerFunc = extern "C" fn() -> !;

fn set_idt_entry(index: usize, handler: HandlerFunc) {
    let pointer = handler as u64;
    unsafe {
        IDT[index] = IDTEntry::new(pointer, 0x08, create_options(true, false));
    }
}

static mut IDT_POINTER: IDTPointer = IDTPointer {
    limit: (IDT_SIZE * core::mem::size_of::<IDTEntry>() - 1) as u16,
    base: 0,
};

pub fn init_idt() {
    unsafe {
        IDT_POINTER.base = &IDT as *const _ as u64;
    }
    
    set_idt_entry(0, divide_by_zero_handler);
    
    unsafe {
        asm!("lidt [{}]", in(reg) &IDT_POINTER);
    }
}

extern "C" fn divide_by_zero_handler() -> ! {
    println!("Divide by zero exception");
    loop {}
}