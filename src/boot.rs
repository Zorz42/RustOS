use core::arch::asm;
use crate::riscv::{get_core_id, get_mhartid, get_mstatus, get_sie, MSTATUS_MACHINE, MSTATUS_SUPERVISOR, set_medeleg, set_mepc, set_mideleg, set_mstatus, set_pmpaddr0, set_pmpcfg0, set_satp, set_sie, set_tp, SIE_EXTERNAL, SIE_SOFTWARE, SIE_TIMER};

use core::arch::global_asm;
use crate::main;
global_asm!(include_str!("entry.S"));

const STACK_SIZE: usize = 4 * 1024; // 4kB
const NUM_CORES: usize = 4;

#[used]
#[no_mangle]
static KERNEL_STACK: [u8; STACK_SIZE * NUM_CORES] = [0; STACK_SIZE * NUM_CORES];

fn putchar(c: u8) {
    let addr = 0x10000000 as *mut u8;
    unsafe {
        while *addr.add(5) & (1 << 5) == 0 {}

        *addr = c;
    }
}

pub fn infinite_loop() -> ! {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

fn main_caller() -> ! {
    if get_core_id() != 0 {
        // every core except the main one goes to the loop here
        infinite_loop();
    }

    main();

    let val = "Going to infinite loop...\n";
    for c in val.as_bytes() {
        putchar(*c);
    }
    infinite_loop();
}

#[no_mangle]
extern "C" fn rust_entry() -> ! {
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

    // load hartid into tp
    set_tp(get_mhartid());

    unsafe {
        asm!("mret");
    }

    infinite_loop();
}