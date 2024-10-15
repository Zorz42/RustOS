use core::arch::asm;
use core::sync::atomic::{fence, Ordering};

pub struct Lock {
    acquired: i32,
}

static mut SPINLOCK_COUNT: u64 = 0;

pub fn get_spinlock_count() -> u64 {
    unsafe {
        SPINLOCK_COUNT
    }
}

unsafe fn amoswap(addr: *mut i32, val: i32) -> i32 {
    let res: i32;
    asm!("amoswap.w {}, {}, ({})", out(reg) res, in(reg) val, in(reg) addr as u64);
    res
}

impl Lock {
    pub const fn new() -> Self {
        Self { acquired: 0 }
    }

    pub fn try_lock(&self) -> bool {
        fence(Ordering::Release);
        let res = unsafe { amoswap(&self.acquired as *const i32 as *mut i32, 1) == 0 };
        fence(Ordering::Release);
        res
    }

    pub fn spinlock(&self) {
        while !self.try_lock() {
            unsafe {
                SPINLOCK_COUNT += 1;
            }
        }
    }

    pub fn unlock(&self) {
        fence(Ordering::Release);
        assert_eq!(unsafe { amoswap(&self.acquired as *const i32 as *mut i32, 0) }, 1);
        fence(Ordering::Release);
    }
}
