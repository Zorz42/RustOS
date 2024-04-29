use crate::memory::{free, malloc};

pub struct Box<T> {
    ptr: *mut T,
}

impl<T> Box<T> {
    pub fn get_type_size() -> usize {
        core::mem::size_of::<T>()
    }

    pub fn new(val: T) -> Self {
        Self {
            ptr: malloc(Self::get_type_size()) as *mut T,
        }
    }

    pub fn new_uninit() -> Self {
        Self { ptr: 0 as *mut T }
    }

    pub unsafe fn get_raw(&mut self) -> *mut T {
        self.ptr
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            if self.ptr != 0 as *mut T {
                free(self.ptr as *mut u8);
            }
        }
    }
}
