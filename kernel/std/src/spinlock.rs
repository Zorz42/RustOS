use core::arch::asm;

pub struct Lock {
    acquired: i32,
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
        unsafe { amoswap(&self.acquired as *const i32 as *mut i32, 1) == 0 }
    }

    pub fn spinlock(&self) {
        while !self.try_lock() {}
    }

    pub fn unlock(&self) {
        assert_eq!(unsafe { amoswap(&self.acquired as *const i32 as *mut i32, 0) }, 1);
    }
}
