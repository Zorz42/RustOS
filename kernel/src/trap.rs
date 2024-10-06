use core::arch::{asm, global_asm};
use crate::riscv::{get_core_id, get_scause, get_sepc, get_sip, get_sstatus, get_stval, interrupts_get, set_sepc, set_sip, set_sstatus, set_stvec, SSTATUS_SPP};
use crate::timer::{get_ticks, tick};
use std::println;
use crate::boot::infinite_loop;
use crate::input::virtio_input_irq;
use crate::plic::{plic_complete, plic_irq};
use crate::program_runner::{get_context, jump_to_program};
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
extern "C" fn usertrap() -> ! {
    //assert_eq!(get_sstatus() & SSTATUS_SPP, 0);
    assert!(!interrupts_get());

    println!("usertrap");
    println!("pc is 0x{:x}", get_context().pc);
    println!("Whole context is {:?}", *get_context());

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
            get_context().pc += 4;
            println!("User interrupt occurred with code {}", get_context().a7);
        }
        InterruptType::Unknown => {
            println!("Interrupt occurred");
            println!("Scause: {}", get_scause());
            println!("Sepc: 0x{:x}", get_sepc());
            println!("Stval: 0x{:x}", get_stval());
            panic!("usertrap");
        }
    }

    // return to kernel now
    set_sstatus(get_sstatus() | SSTATUS_SPP);
    // set sepc to loop_inf
    set_sepc(loop_inf as *const () as u64);
    switch_to_kernel_trap();

    let stack_pointer = 128 * 1024 * 1024 + 0x80000000 - 64 * 1024 * get_core_id();

    unsafe {
        asm!(r#"
        mv sp, {0}
        sret
        "#, in(reg) stack_pointer);
    }

    infinite_loop()
}

fn loop_inf() -> ! {
    loop {
        if get_ticks() % 1000 == 0 {
            println!("Ticks: {}", get_ticks());
        }
    }
}
