use core::ptr::addr_of;
use core::sync::atomic::{fence, Ordering};
use crate::riscv::{amoswap, get_core_id};

pub struct KernelLock {
    acquired: i32,
    locked_by: i32, // which core locked it?
}

static mut KERN_SPINLOCK_COUNT: u64 = 0;

pub fn get_kern_spinlock_count() -> u64 {
    unsafe {
        KERN_SPINLOCK_COUNT
    }
}

impl KernelLock {
    pub const fn new() -> Self {
        Self { acquired: 0, locked_by: 0 }
    }

    pub fn try_lock(&self) -> bool {
        fence(Ordering::Release);
        let res = unsafe { amoswap(&self.acquired as *const i32 as *mut i32, 1) == 0 };
        fence(Ordering::Release);
        if res {
            unsafe {
                let addr = addr_of!(self.locked_by) as *mut i32;
                *addr = get_core_id() as i32;
            }
        }
        res
    }

    pub fn spinlock(&self) {
        while !self.try_lock() {

        }
    }

    pub fn unlock(&self) {
        fence(Ordering::Release);
        assert_eq!(unsafe { amoswap(&self.acquired as *const i32 as *mut i32, 0) }, 1);
        fence(Ordering::Release);
    }

    pub const fn locked_by(&self) -> i32 {
        if self.acquired == 0 {
            -1
        } else {
            self.locked_by
        }
    }
}
