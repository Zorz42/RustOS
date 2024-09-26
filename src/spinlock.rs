use core::ptr::addr_of;
use std::println;
use crate::riscv::{amoswap, get_core_id};
use crate::timer::get_ticks;

pub struct Lock {
    acquired: i32,
    locked_by: i32, // which core locked it?
}

impl Lock {
    pub const fn new() -> Self {
        Self { acquired: 0, locked_by: 0 }
    }

    pub fn try_lock(&self) -> bool {
        let res = unsafe { amoswap(&self.acquired as *const i32 as *mut i32, 1) == 0 };
        if res {
            unsafe {
                let addr = addr_of!(self.locked_by) as *mut i32;
                *addr = get_core_id() as i32;
            }
        }
        res
    }

    pub fn spinlock(&self) {
        while !self.try_lock() {}
    }

    pub fn unlock(&self) {
        assert_eq!(unsafe { amoswap(&self.acquired as *const i32 as *mut i32, 0) }, 1);
    }

    pub const fn locked_by(&self) -> i32 {
        if self.acquired == 0 {
            -1
        } else {
            self.locked_by
        }
    }
}
