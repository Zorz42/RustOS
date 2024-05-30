use crate::riscv::amoswap;

pub struct Lock {
    acquired: i32,
}

impl Lock {
    pub const fn new() -> Self {
        Self {
            acquired: 0,
        }
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