use crate::pointer::Ptr;
use core::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct Box<T> {
    ptr: Ptr<T>,
}

impl<T> Box<T> {
    pub fn new(val: T) -> Self {
        let mut ptr = unsafe { Ptr::new(1) };
        unsafe {
            core::ptr::write(ptr.get_mut(), val);
        }
        Self { ptr }
    }

    pub unsafe fn get_raw(&mut self) -> *mut T {
        self.ptr.get_mut()
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            drop(core::ptr::read(self.ptr.get_mut()));
        }
    }
}

impl<T: Default> Default for Box<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr.get() }
    }
}

impl<T> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut (*self.ptr.get_mut()) }
    }
}

impl<T: Clone> Clone for Box<T> {
    fn clone(&self) -> Self {
        Self::new(self.deref().clone())
    }
}

impl<T: PartialEq> PartialEq for Box<T> {
    fn eq(&self, other: &Box<T>) -> bool {
        self.deref() == other.deref()
    }
}
