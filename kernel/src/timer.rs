use crate::boot::NUM_CORES;
use crate::riscv::{get_mhartid, get_mie, get_mstatus, set_mie, set_mscratch, set_mstatus, set_mtvec, CLINT, MIE_TIMER, MSTATUS_MMI};
use std::Lock;
use core::ptr::{addr_of, write_volatile};

extern "C" {
    fn timervec();
}

// a scratch area per CPU for machine-mode timer interrupts.
#[used]
static mut TIMER_SCRATCH: [u64; NUM_CORES * 5] = [0; NUM_CORES * 5];
static TIMER_LOCK: Lock = Lock::new();

pub fn machine_mode_timer_init() {
    TIMER_LOCK.spinlock();

    let frequency = 100;
    let interval = 1193180 / frequency;
    unsafe {
        write_volatile((CLINT + 0x4000 + 8 * get_mhartid()) as *mut u64, *((CLINT + 0xBFF8) as *mut u64) + interval);
    }

    unsafe {
        TIMER_SCRATCH[(5 * get_mhartid() + 3) as usize] = CLINT + 0x4000 + 8 * get_mhartid();
        TIMER_SCRATCH[(5 * get_mhartid() + 4) as usize] = interval;

        set_mscratch(addr_of!(TIMER_SCRATCH[(5 * get_mhartid()) as usize]) as u64);
    }

    set_mtvec(timervec as u64);

    // enable machine mode interrupts
    let mut mstatus = get_mstatus();
    mstatus |= MSTATUS_MMI;
    set_mstatus(mstatus);

    let mut mie = get_mie();
    mie |= MIE_TIMER;
    set_mie(mie);

    TIMER_LOCK.unlock();
}

static mut TICKS: u64 = 0;

// this is called every tick on core 0
pub fn tick() {
    unsafe {
        TICKS += 1;
    }
}

pub fn get_ticks() -> u64 {
    unsafe { TICKS }
}
