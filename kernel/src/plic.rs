use crate::riscv::get_core_id;

const PLIC_BASE: u64 = 0x0c000000;

pub fn plicinit() {
    for irq in 0..=8 {
        let addr = (PLIC_BASE + 4 * irq) as *mut u32;
        unsafe {
            addr.write_volatile(1);
        }
    }
}

pub fn plicinithart() {
    let val = 0b111111111;
    let addr1 = (PLIC_BASE + 0x2080 + get_core_id() * 0x100) as *mut u32;
    unsafe {
        addr1.write_volatile(val);
    }

    let addr2 = (PLIC_BASE + 0x201000 + get_core_id() * 0x2000) as *mut u32;
    unsafe {
        addr2.write_volatile(0);
    }
}

pub fn plic_irq() -> u32 {
    let addr = (PLIC_BASE + 0x201004 + get_core_id() * 0x2000) as *mut u32;
    unsafe { addr.read_volatile() }
}

pub fn plic_complete(irq: u32) {
    let addr = (PLIC_BASE + 0x201004 + get_core_id() * 0x2000) as *mut u32;
    unsafe {
        addr.write_volatile(irq);
    }
}