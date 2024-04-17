use core::arch::asm;
use crate::ports::byte_out;
use crate::println;

#[macro_export]
macro_rules! interrupt_wrapper {
    ($name: ident) => {{
        #[naked]
        extern "C" fn wrapper() -> ! {
            unsafe {
                // call the handler via assembly call
                // preserve registers
                asm!("
                push rax
                push rcx
                push rdx
                push rsi
                push rdi
                push r8
                push r9
                push r10
                push r11

                call {}

                pop r11
                pop r10
                pop r9
                pop r8
                pop rdi
                pop rsi
                pop rdx
                pop rcx
                pop rax

                iretq
                ", 
                sym $name, 
                options(noreturn));
            }
        }
        wrapper
    }}
}


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

pub fn set_idt_entry(index: usize, handler: HandlerFunc) {
    let pointer = handler as u64;
    unsafe {
        IDT[index] = IDTEntry::new(pointer, 0x08, create_options(true, false));
    }
}

static mut IDT_POINTER: IDTPointer = IDTPointer {
    limit: (IDT_SIZE * core::mem::size_of::<IDTEntry>() - 1) as u16,
    base: 0,
};

const exception_messages: [&str; 32] = [
    "Divide by zero",
    "Debug",
    "Non-maskable interrupt",
    "Breakpoint",
    "Overflow",
    "Bound range exceeded",
    "Invalid opcode",
    "Device not available",
    "Double fault",
    "Coprocessor segment overrun",
    "Invalid TSS",
    "Segment not present",
    "Stack-segment fault",
    "General protection fault",
    "Page fault",
    "Reserved",
    "x87 FPU floating-point error",
    "Alignment check",
    "Machine check",
    "SIMD floating-point",
    "Virtualization",
    "Control",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Security",
    "Reserved",
];

pub fn init_idt() {
    unsafe {
        IDT_POINTER.base = &IDT as *const _ as u64;
    }
    
    for i in 0..32 {
        set_idt_entry(i, (|| -> ! {
                println!("{} exception", exception_messages[i]);
                loop {
                    unsafe {
                        asm!("hlt");
                    }
                }
            })()
        );
    }
    
    
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
        asm!("lidt [{}]", in(reg) &IDT_POINTER);
    }
}

extern "C" fn divide_by_zero_handler() -> ! {
    println!("Divide by zero exception");
    loop {}
}

extern "C" fn breakpoint_handler() -> ! {
    println!("Breakpoint exception");
    loop {}
}

extern "C" fn invalid_opcode_handler() -> ! {
    println!("Invalid opcode exception");
    loop {}
}

extern "C" fn page_fault_handler() -> ! {
    println!("Page fault exception");
    loop {}
}