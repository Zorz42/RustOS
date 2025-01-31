use crate::riscv::{get_mcounteren, get_menvcfg, get_mhartid, get_mstatus, get_sie, get_sstatus, set_mcounteren, set_medeleg, set_menvcfg, set_mepc, set_mideleg, set_mstatus, set_pmpaddr0, set_pmpcfg0, set_satp, set_sie, set_sstatus, set_tp, MSTATUS_MACHINE, MSTATUS_SUPERVISOR, SIE_EXTERNAL, SIE_SOFTWARE, SIE_TIMER, SSTATUS_SUM};
use core::arch::asm;

use crate::main;
use crate::timer::machine_mode_timer_init;
use core::arch::global_asm;

global_asm!(include_str!("asm/entry.S"));

pub const STACK_SIZE: usize = 64 * 1024; // 64kB
pub const NUM_CORES: usize = 4;

pub fn infinite_loop() -> ! {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

fn main_caller() -> ! {
    main();

    infinite_loop();
}

#[no_mangle]
extern "C" fn rust_entry() -> ! {
    assert!(get_mhartid() < NUM_CORES as u64);

    // set to MODE_SUPERVISOR from MODE_MACHINE
    let mut mstatus = get_mstatus();
    mstatus &= !MSTATUS_MACHINE;
    mstatus |= MSTATUS_SUPERVISOR;
    set_mstatus(mstatus);

    // set the return address to main caller after mret
    set_mepc(main_caller as *const () as u64);

    // disable paging
    set_satp(0);

    // set interrupts and exceptions to machine mode
    set_medeleg(0xFFFF);
    set_mideleg(0xFFFF);
    let mut sie = get_sie();
    sie |= SIE_EXTERNAL;
    sie |= SIE_SOFTWARE;
    sie |= SIE_TIMER;
    set_sie(sie);

    // give kernel whole memory
    set_pmpaddr0(0x3fffffffffffff);
    set_pmpcfg0(0xF);

    set_sstatus(get_sstatus() | SSTATUS_SUM);

    machine_mode_timer_init();

    // enable the sstc extension (i.e. stimecmp).
    set_menvcfg(get_menvcfg() | (1u64 << 63));

    // allow supervisor to use cycle.
    set_mcounteren(get_mcounteren() | 7);

    // load hartid into tp
    set_tp(get_mhartid());

    unsafe {
        asm!("mret");
    }

    infinite_loop();
}
