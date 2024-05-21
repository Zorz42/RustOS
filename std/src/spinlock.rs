use core::ops::{Deref, DerefMut};

pub struct SpinLock<T> {
    obj: T,
    locked: bool,
}

pub struct Lock<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> SpinLock<T> {
    pub const fn new(obj: T) -> Self {
        Self {
            obj,
            locked: false,
        }
    }
    
    pub fn get(&self) -> Lock<T> {
        while self.locked {}
        
        Lock {
            lock: self,
        }
    }
}

impl<'a, T> Deref for Lock<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*core::ptr::addr_of!(self.lock.obj) }
    }
}

impl<'a, T> DerefMut for Lock<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let a = core::ptr::addr_of!(self.lock.obj);
        let addr = a as *mut T;
        unsafe { addr.as_mut_unchecked() }
    }
}

impl<'a, T> Drop for Lock<'a, T> {
    fn drop(&mut self) {
        let a = self.lock as *const SpinLock<T> as *mut SpinLock<T>;
        unsafe {
            (*a).locked = false;
        }
        
    }
}

unsafe impl<T> Sync for SpinLock<T> {
    
}