use core::arch::asm;

use paste::paste;

// which core am I?
pub fn get_core_id() -> u64 {
    let res: u64;
    unsafe {
        asm!("csrr {}, mhartid", out(reg) res);
    }
    res
}

pub const MODE_MACHINE: u64 = 3 << 11;
pub const MODE_SUPERVISOR: u64 = 1 << 11;
pub const MODE_USER: u64 = 0 << 11;

macro_rules! csr_getter_setter {
    ($csr_name:ident) => {
        paste! {
            pub fn [<get_ $csr_name>]() -> u64 {
                let res: u64;
                unsafe {
                    asm!(concat!("csrr {}, ", stringify!($csr_name)), out(reg) res);
                }
                res
            }

            pub fn [<set_ $csr_name>](val: u64) {
                unsafe {
                    asm!(concat!("csrw ", stringify!($csr_name), ", {}"), in(reg) val);
                }
            }
        }
    };
}

// mstatus register holds info about execution mode
//csr_getter_setter!(mstatus);

// satp register hold the pointer to the page table
//csr_getter_setter!(satp);