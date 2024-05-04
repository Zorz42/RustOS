use core::ops::{Deref, DerefMut};
use crate::pointer::Ptr;
use crate::swap;

#[derive(Debug)]
pub struct Box<T> {
    ptr: Ptr<T>,
}

impl<T> Box<T> {
    pub fn new(mut val: T) -> Self {
        let mut ptr = Ptr::new(1);
        unsafe {
            swap(&mut *ptr.get_mut(), &mut val);
        }
        Self {
            ptr,
        }
    }

    pub unsafe fn new_uninit() -> Self {
        Self { ptr: Ptr::new(1) }
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