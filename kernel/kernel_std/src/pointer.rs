use crate::{free, malloc};

#[derive(Debug)]
pub struct Ptr<T> {
    ptr: *mut T,
}

impl<T> Ptr<T> {
    pub unsafe fn new(size: usize) -> Self {
        Self {
            ptr: malloc(size * size_of::<T>()) as *mut T,
        }
    }

    pub const unsafe fn new_empty() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
        }
    }

    pub fn get(&self) -> *const T {
        self.ptr
    }

    pub fn get_mut(&mut self) -> *mut T {
        self.ptr
    }
}

#[allow(dead_code)]
impl<T: Default> Ptr<T> {
    pub fn new_default(size: usize) -> Self {
        let res = unsafe { Self::new(size) };
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
            if self.ptr != 0 as *mut T {
                free(self.ptr as *mut u8);
            }
        }
    }
}
