use core::ops::{Deref, DerefMut};
use crate::pointer::Ptr;

pub struct Box<T> {
    ptr: Ptr<T>,
}

impl<T> Box<T> {
    pub fn new(val: T) -> Self {
        let mut ptr = Ptr::new(1);
        unsafe {
            *ptr.get_mut() = val;
        }
        Self {
            ptr,
        }
    }

    pub fn new_uninit() -> Self {
        Self { ptr: Ptr::new_empty() }
    }

    pub unsafe fn get_raw(&mut self) -> *mut T {
        self.ptr.get_mut()
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