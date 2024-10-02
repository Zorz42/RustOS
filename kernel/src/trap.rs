use crate::riscv::{get_core_id, get_scause, get_sepc, get_sip, get_sstatus, get_stval, interrupts_get, set_sip, set_stvec, SSTATUS_SPP};
use crate::timer::tick;
use std::println;
use crate::input::virtio_input_irq;
use crate::plic::{plic_complete, plic_irq};
use crate::virtio::device::virtio_irq;

extern "C" {
    fn kernelvec();
}

#[no_mangle]
extern "C" fn kerneltrap() {
    assert_ne!(get_sstatus() & SSTATUS_SPP, 0);
    assert!(!interrupts_get());

    let ty = get_interrupt_type();

    if ty == InterruptType::Unknown {
        println!("Interrupt occurred");
        println!("Scause: {}", get_scause());
        println!("Sepc: 0x{:x}", get_sepc());
        println!("Stval: 0x{:x}", get_stval());
        panic!("kerneltrap");
    }
}

#[derive(PartialEq)]
enum InterruptType {
    Unknown,
    Timer,
    OtherDevice,
}

fn get_interrupt_type() -> InterruptType {
    let scause = get_scause();

    if (scause & 0x8000000000000000) != 0 && (scause & 0xff) == 9 {
        let irq = plic_irq();

        virtio_irq(irq);
        virtio_input_irq(irq);

        plic_complete(irq);

        InterruptType::OtherDevice

    } else if scause == 0x8000000000000001 {
        if get_core_id() == 0 {
            tick();
        }

        // acknowledge the software interrupt by clearing
        // the SSIP bit in sip.
        let mut sip = get_sip();
        sip &= !2;
        set_sip(sip);

        InterruptType::Timer
    } else {
        InterruptType::Unknown
    }
}

pub fn init_trap() {
    set_stvec(kernelvec as u64);
}
