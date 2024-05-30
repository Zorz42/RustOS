use core::arch::asm;

use paste::paste;

macro_rules! csr_get {
    ($csr_name:ident) => {
        paste! {
            pub fn [<get_ $csr_name>]() -> u64 {
                let res: u64;
                unsafe {
                    asm!(concat!("csrr {}, ", stringify!($csr_name)), out(reg) res);
                }
                res
            }
        }
    };
}

macro_rules! csr_set {
    ($csr_name:ident) => {
        paste! {

            pub fn [<set_ $csr_name>](val: u64) {
                unsafe {
                    asm!(concat!("csrw ", stringify!($csr_name), ", {}"), in(reg) val);
                }
            }
        }
    };
}

macro_rules! csr_get_set {
    ($csr_name:ident) => {
        csr_get!($csr_name);
        csr_set!($csr_name);
    };
}

// mhartid holds the number of the current core
csr_get!(mhartid);

// mstatus register holds info about execution mode
pub const MSTATUS_MACHINE: u64 = 3 << 11;
pub const MSTATUS_SUPERVISOR: u64 = 1 << 11;
pub const MSTATUS_USER: u64 = 0 << 11;
csr_get_set!(mstatus);

// satp register holds the pointer to the page table
csr_get_set!(satp);

// mepc register holds the program counter for mret
csr_get_set!(mepc);

// Machine Exception Delegation
csr_get_set!(medeleg);

// Machine Interrupt Delegation
csr_get_set!(mideleg);

// Supervisor Interrupt Enable
pub const SIE_EXTERNAL: u64 = 1 << 9;
pub const SIE_TIMER: u64 = 1 << 5;
pub const SIE_SOFTWARE: u64 = 1 << 1;
csr_get_set!(sie);

// Physical memory protection config register
csr_get_set!(pmpcfg0);

// Physical memory protection address register
csr_get_set!(pmpaddr0);

// Thread Pointer we will use to store hartid (like in xv6)
pub fn get_tp() -> u64 {
    let mut res: u64 = 0;
    unsafe {
        asm!(concat!("mv {}, tp"), out(reg) res);
    }
    res
}

pub fn set_tp(val: u64) {
    unsafe {
        asm!(concat!("mv tp, {}"), in(reg) val);
    }
}

pub fn get_core_id() -> u64 {
    get_tp()
}

// amoswap does *addr = val and also returns *addr before the change
// it does it in one instruction and is used for locks
pub unsafe fn amoswap(addr: *mut i32, val: i32) -> i32 {
    let mut res: i32 = 0;
    asm!("amoswap.w {}, {}, ({})", out(reg) res, in(reg) val, in(reg) addr as u64);
    res
}