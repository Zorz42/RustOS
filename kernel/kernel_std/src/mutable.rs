use core::cell::{UnsafeCell};
use crate::spinlock::Lock;

// container for mutable data that is static
pub struct Mutable<T> {
    lock: Lock,
    data: UnsafeCell<T>,
    curr_token: UnsafeCell<u32>,
}

pub struct MutableToken {
    token: u32,
}

impl<T> Mutable<T> {
    pub const fn new(data: T) -> Self {
        Self { lock: Lock::new(), data: UnsafeCell::new(data), curr_token: UnsafeCell::new(0) }
    }

    pub fn borrow(&self) -> MutableToken {
        self.lock.spinlock();
        let curr_token_mut = unsafe { &mut *(self.curr_token.get()) };
        *curr_token_mut = curr_token_mut.wrapping_add(1);
        let token = unsafe {
            *self.curr_token.get()
        };
        MutableToken { token }
    }

    pub fn get(&self, token: &MutableToken) -> &T {
        #[cfg(assertions)]
        assert_eq!(token.token, unsafe { *self.curr_token.get() });
        unsafe { &*self.data.get() }
    }

    pub fn get_mut(&self, token: &MutableToken) -> &mut T {
        #[cfg(assertions)]
        assert_eq!(token.token, unsafe { *self.curr_token.get() });
        unsafe { &mut *(self.data.get()) }
    }

    pub fn release(&self, token: MutableToken) {
        #[cfg(assertions)]
        assert_eq!(token.token, unsafe { *self.curr_token.get() });
        self.lock.unlock();
    }
}

// Mutable is sync (because it uses spinlock)
unsafe impl<T> Sync for Mutable<T> {}