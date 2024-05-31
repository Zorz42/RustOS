use core::arch::global_asm;
use crate::boot::NUM_CORES;
use crate::riscv::{CLINT, get_mhartid, get_mie, get_mstatus, MIE_TIMER, MSTATUS_MMI, set_mie, set_mscratch, set_mstatus, set_mtvec};
global_asm!(include_str!("asm/kernelvec.S"));

extern "C" {
    fn timervec();
}

// a scratch area per CPU for machine-mode timer interrupts.
static TIMER_SCRATCH: [u64; NUM_CORES * 5] = [0; NUM_CORES * 5];

pub fn machine_mode_timer_init() {
    let interval = 10000; // about 1/10 th of a second in qemu
    unsafe {
        *((CLINT + 0x4000 + 8 * get_mhartid()) as *mut u64) = *((CLINT + 0xBFF8) as *mut u64) + interval;
    }

    let addr = unsafe { core::ptr::addr_of!(TIMER_SCRATCH[0]).add((5 * get_mhartid()) as usize) as *mut u64 };
    unsafe {
        *addr.add(3) = CLINT + 0x4000 + 8 * get_mhartid();
        *addr.add(4) = interval;
    }
    set_mscratch(addr as u64);

    set_mtvec(timervec as u64);

    // enable machine mode interrupts
    let mut mstatus = get_mstatus();
    mstatus |= MSTATUS_MMI;
    set_mstatus(mstatus);

    let mut mie = get_mie();
    mie |= MIE_TIMER;
    set_mie(mie);
}

static mut TICKS: u64 = 0;

// this is called every tick on core 0
pub fn tick() {
    unsafe {
        TICKS += 1;
    }
}

pub fn get_ticks() -> u64 {
    unsafe {
        TICKS
    }
}