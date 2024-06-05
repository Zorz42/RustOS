use crate::riscv::get_core_id;

const PLIC_BASE: u64 = 0x0c000000;

pub fn plicinit() {
    for irq in 0..=8 {
        unsafe {
            *(PLIC_BASE as *mut u32).add(irq) = 1;
        }
    }
}

pub fn plicinithart() {
    let val = 0b111111111;
    let addr1 = PLIC_BASE + 0x2080 + get_core_id() * 0x100;
    unsafe {
        *(addr1 as *mut u32) = val;
    }

    let addr2 = PLIC_BASE + 0x201000 + get_core_id() * 0x2000;
    unsafe {
        *(addr2 as *mut u32) = 0;
    }
}

pub fn plic_irq() -> u32 {
    let addr = PLIC_BASE + 0x201004 + get_core_id() * 0x2000;
    unsafe { *(addr as *mut u32) }
}

pub fn plic_complete(irq: u32) {
    unsafe {
        *((PLIC_BASE + 0x201004 + get_core_id() * 0x2000) as *mut u32) = irq;
    }
}