use core::arch::global_asm;
use crate::riscv::{get_core_id, get_scause, get_sepc, get_sip, get_sstatus, get_stval, interrupts_enable, interrupts_get, set_sip, set_sstatus, set_stvec, SSTATUS_SPP, SSTATUS_UIE};
use crate::timer::{get_ticks, tick};
use kernel_std::{debug_str, debugln, print, println};
use crate::input::virtio_input_irq;
use crate::memory::{free_page, map_page_auto, switch_to_page_table};
use crate::plic::{plic_complete, plic_irq};
use crate::print::check_screen_refresh_for_print;
use crate::scheduler::{get_context, get_cpu_data, mark_process_ready, put_process_to_sleep, refresh_paging_for_proc, scheduler, scheduler_next_proc, terminate_process};
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

            scheduler_next_proc();
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
    assert_eq!(get_sstatus() & SSTATUS_SPP, 0);
    assert!(!interrupts_get());

    let ty = get_interrupt_type();
    get_cpu_data().was_last_interrupt_external = false;

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

            scheduler_next_proc();
        }
        InterruptType::OtherDevice => {
            let irq = plic_irq();

            virtio_irq(irq);
            virtio_input_irq(irq);

            plic_complete(irq);
        }
        InterruptType::User => {
            get_context().pc += 4;
            get_cpu_data().was_last_interrupt_external = true;
        }
        InterruptType::Unknown => {
            debug_str("Interrupt occurred");
            println!("Interrupt occurred");
            println!("Scause: {}", get_scause());
            println!("Sepc: 0x{:x}", get_sepc());
            println!("Stval: 0x{:x}", get_stval());
            panic!("usertrap");
        }
    }

    switch_to_kernel_trap();

    // set bit in sstatus
    set_sstatus(get_sstatus() | SSTATUS_SPP);

    // clear user interrupt enable
    set_sstatus(get_sstatus() & !SSTATUS_UIE);

    interrupts_enable(true);

    sched_resume()
}

fn sched_resume() -> ! {
    if get_cpu_data().was_last_interrupt_external {
        let int_code = get_context().a7;
        match int_code {
            1 => {
                // print str
                let arg1 = get_context().a3 as *mut u8;
                let arg2 = get_context().a4;
                // convert to &str
                let arg1 = unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(arg1, arg2 as usize)) };

                print!("{}", arg1);
                mark_process_ready(get_cpu_data().last_pid);
            }
            2 => {
                // get ticks
                get_context().a2 = get_ticks();
                mark_process_ready(get_cpu_data().last_pid);
            }
            3 => {
                // get pid
                get_context().a2 = get_cpu_data().last_pid as u64;
                mark_process_ready(get_cpu_data().last_pid);
            }
            4 => {
                // exit process
                terminate_process(get_cpu_data().last_pid);
            }
            5 => {
                // Alloc page
                let addr = get_context().a3 as *mut u8;
                let ignore_if_exists = get_context().a4 != 0;
                map_page_auto(addr, ignore_if_exists, true, true, false);
                mark_process_ready(get_cpu_data().last_pid);
            }
            6 => {
                // Dealloc page
                let addr = get_context().a3 as *mut u8;
                free_page(addr as u64);
                refresh_paging_for_proc(get_cpu_data().last_pid);
                mark_process_ready(get_cpu_data().last_pid);
            }
            7 => {
                // Sleep
                let until = get_ticks() + get_context().a3;
                put_process_to_sleep(get_cpu_data().last_pid, until);
            }
            _ => {
                println!("Unknown user interrupt occurred with code {}", int_code);
            }
        }
    } else {
        mark_process_ready(get_cpu_data().last_pid);
    }
    check_screen_refresh_for_print();
    scheduler()
}
