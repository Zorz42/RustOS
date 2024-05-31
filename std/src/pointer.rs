use crate::{free, malloc};

#[derive(Debug)]
pub struct Ptr<T> {
    ptr: *mut T,
}

impl<T> Ptr<T> {
    pub fn new(size: usize) -> Self {
        Self {
            ptr: malloc(size * core::mem::size_of::<T>()) as *mut T,
        }
    }

    pub fn get(&self) -> *const T {
        self.ptr
    }

    pub fn get_mut(&mut self) -> *mut T {
        self.ptr
    }

    pub fn is_some(&self) -> bool {
        self.ptr != 0 as *mut T
    }
}

impl<T: Default> Ptr<T> {
    pub fn new_default(size: usize) -> Self {
        let res = Self::new(size);
        for i in 0..size {
            unsafe {
                *res.ptr.add(i) = T::default();
            }
        }
        res
    }
}

impl<T> Drop for Ptr<T> {
    fn drop(&mut self) {
        unsafe {
            free(self.ptr as *mut u8);
        }
    }
}
