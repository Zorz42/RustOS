use core::arch::global_asm;
use crate::riscv::{get_core_id, get_scause, get_sepc, get_sip, get_sstatus, get_stval, interrupts_get, set_sip, set_stvec, SSTATUS_SPP};
use crate::timer::tick;
use std::println;
use crate::input::virtio_input_irq;
use crate::plic::{plic_complete, plic_irq};
use crate::virtio::device::virtio_irq;

global_asm!(include_str!("asm/kernelvec.S"));
global_asm!(include_str!("asm/uservec.S"));

extern "C" {
    fn kernelvec();
    fn uservec();
}

#[no_mangle]
extern "C" fn kerneltrap() {
    assert_ne!(get_sstatus() & SSTATUS_SPP, 0);
    assert!(!interrupts_get());

    let ty = get_interrupt_type();

    match ty {
        InterruptType::Timer => {
            if get_core_id() == 0 {
                tick();
            }

            // acknowledge the software interrupt by clearing
            // the SSIP bit in sip.
            let mut sip = get_sip();
            sip &= !2;
            set_sip(sip);
        }
        InterruptType::OtherDevice => {
            let irq = plic_irq();

            virtio_irq(irq);
            virtio_input_irq(irq);

            plic_complete(irq);
        }
        InterruptType::Unknown | InterruptType::User => {
            println!("Interrupt occurred");
            println!("Scause: {}", get_scause());
            println!("Sepc: 0x{:x}", get_sepc());
            println!("Stval: 0x{:x}", get_stval());
            panic!("kerneltrap");
        }
    }
}

enum InterruptType {
    Unknown,
    Timer,
    User,
    OtherDevice,
}

fn get_interrupt_type() -> InterruptType {
    let scause = get_scause();

    if (scause & 0x8000000000000000) != 0 && (scause & 0xff) == 9 {
        InterruptType::OtherDevice
    } else if scause == 0x8000000000000001 {
        InterruptType::Timer
    } else if scause == 8 {
        InterruptType::User
    } else {
        InterruptType::Unknown
    }
}

pub fn switch_to_kernel_trap() {
    set_stvec(kernelvec as u64);
}

pub fn switch_to_user_trap() {
    set_stvec(uservec as u64);
}

#[no_mangle]
extern "C" fn usertrap() {
    assert_eq!(get_sstatus() & SSTATUS_SPP, 0);
    assert!(!interrupts_get());

    let ty = get_interrupt_type();

    match ty {
        InterruptType::Timer => {
            if get_core_id() == 0 {
                tick();
            }

            // acknowledge the software interrupt by clearing
            // the SSIP bit in sip.
            let mut sip = get_sip();
            sip &= !2;
            set_sip(sip);
        }
        InterruptType::OtherDevice => {
            let irq = plic_irq();

            virtio_irq(irq);
            virtio_input_irq(irq);

            plic_complete(irq);
        }
        InterruptType::User => {
            println!("User interrupt occurred");
        }
        InterruptType::Unknown => {
            println!("Interrupt occurred");
            println!("Scause: {}", get_scause());
            println!("Sepc: 0x{:x}", get_sepc());
            println!("Stval: 0x{:x}", get_stval());
            panic!("usertrap");
        }
    }
}
