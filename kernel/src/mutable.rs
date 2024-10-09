use core::cell::{RefCell, UnsafeCell};
use crate::spinlock::Lock;

// container for mutable data that is static
pub struct Mutable<T> {
    lock: Lock,
    data: UnsafeCell<T>,
    curr_token: UnsafeCell<u32>,
}

impl<T> Mutable<T> {
    pub fn new(data: T) -> Self {
        Self { lock: Lock::new(), data: UnsafeCell::new(data), curr_token: UnsafeCell::new(0) }
    }

    pub fn borrow(&self) -> u32 {
        self.lock.spinlock();
        let curr_token_mut = unsafe { &mut *(self.curr_token.get()) };
        *curr_token_mut = curr_token_mut.wrapping_add(1);
        unsafe {
            *self.curr_token.get()
        }
    }

    pub fn get(&self, token: u32) -> &T {
        assert_eq!(token, unsafe { *self.curr_token.get() });
        unsafe { &*self.data.get() }
    }

    pub fn get_mut(&self, token: u32) -> &mut T {
        assert_eq!(token, unsafe { *self.curr_token.get() });
        unsafe { &mut *(self.data.get()) }
    }

    pub fn release(&self, token: u32) {
        assert_eq!(token, unsafe { *self.curr_token.get() });
        self.lock.unlock();
    }
}