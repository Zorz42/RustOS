use crate::riscv::{get_core_id, get_scause, get_sip, get_sstatus, interrupts_get, set_sip, set_stvec, SSTATUS_SPP};
use crate::timer::tick;
use crate::trap::InterruptType::{OtherDevice, Timer};

extern "C" {
    fn kernelvec();
}

#[no_mangle]
extern "C" fn kerneltrap() {
    assert_ne!(get_sstatus() & SSTATUS_SPP, 0);
    assert!(!interrupts_get());

    let ty = get_interrupt_type();
    assert!(ty != InterruptType::Unknown);
}

#[derive(PartialEq)]
enum InterruptType {
    Unknown,
    Timer,
    OtherDevice,
}

fn get_interrupt_type() -> InterruptType {
    let scause = get_scause();

    if (scause & 0x8000000000000000) != 0 &&
        (scause & 0xff) == 9 {
        todo!();
    } else if scause == 0x8000000000000001 {
        if get_core_id() == 0 {
            tick();
        }

        // acknowledge the software interrupt by clearing
        // the SSIP bit in sip.
        let mut sip = get_sip();
        sip &= !2;
        set_sip(sip);

        Timer
    } else {
        OtherDevice
    }
}

pub fn init_trap() {
    set_stvec(kernelvec as u64);
}