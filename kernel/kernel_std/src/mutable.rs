use core::cell::{UnsafeCell};
use crate::spinlock::Lock;

// container for mutable data that is static
pub struct Mutable<T> {
    lock: Lock,
    data: UnsafeCell<T>,
    curr_token: UnsafeCell<u32>,
}

pub struct MutableToken {
    _token: u32,
}

impl<T> Mutable<T> {
    pub const fn new(data: T) -> Self {
        Self { lock: Lock::new(), data: UnsafeCell::new(data), curr_token: UnsafeCell::new(0) }
    }

    pub fn try_borrow(&self) -> Option<MutableToken> {
        if self.lock.try_lock() {
            let curr_token_mut = unsafe { &mut *(self.curr_token.get()) };
            *curr_token_mut = curr_token_mut.wrapping_add(1);
            let token = unsafe {
                *self.curr_token.get()
            };
            Some(MutableToken { _token: token })
        } else {
            None
        }
    }

    pub fn borrow(&self) -> MutableToken {
        loop {
            if let Some(token) = self.try_borrow() {
                return token;
            }
        }
    }

    pub fn get(&self, _token: &MutableToken) -> &T {
        #[cfg(feature = "assertions")]
        assert_eq!(_token._token, unsafe { *self.curr_token.get() });
        unsafe { &*self.data.get() }
    }

    pub fn get_mut(&self, _token: &MutableToken) -> &mut T {
        #[cfg(feature = "assertions")]
        assert_eq!(_token._token, unsafe { *self.curr_token.get() });
        unsafe { &mut *(self.data.get()) }
    }

    pub fn release(&self, _token: MutableToken) {
        #[cfg(feature = "assertions")]
        assert_eq!(_token._token, unsafe { *self.curr_token.get() });
        self.lock.unlock();
    }
}

// Mutable is sync (because it uses spinlock)
unsafe impl<T> Sync for Mutable<T> {}