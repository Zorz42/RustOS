use core::mem::MaybeUninit;
use core::ops::DerefMut;
use crate::memcpy;
use crate::pointer::Ptr;
use crate::utils::swap;

pub struct Vec<T> {
    arr: Ptr<T>,
    size: usize,
    capacity: usize,
}

impl<T> Vec<T> {
    pub fn new() -> Self {
        let capacity = 1;
        Self {
            capacity,
            size: 0,
            arr: Ptr::new(capacity),
        }
    }

    pub unsafe fn get_unchecked(&self, i: usize) -> &T {
        &*self.arr.get().add(i)
    }

    pub unsafe fn get_mut_unchecked(&mut self, i: usize) -> &mut T {
        &mut *self.arr.get_mut().add(i)
    }

    pub fn get(&self, i: usize) -> Option<&T> {
        if i < self.size {
            unsafe {
                Some(self.get_unchecked(i))
            }
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        if i < self.size {
            unsafe {
                Some(self.get_mut_unchecked(i))
            }
        } else {
            None
        }
    }

    fn double_capacity(&mut self) {
        let mut new_arr = Ptr::new(self.capacity * 2);
        unsafe {
            memcpy(self.arr.get_mut() as *mut u8, new_arr.get_mut() as *mut u8, (self.size * core::mem::size_of::<T>() + 7) / 8 * 8);
        }
        self.arr = new_arr;
        self.capacity *= 2;
    }

    pub fn reserve(&mut self, size: usize) {
        while self.capacity < size {
            self.double_capacity();
        }
    }

    pub fn push(&mut self, element: T) {
        self.size += 1;
        self.reserve(self.size);
        unsafe {
            *self.get_mut_unchecked(self.size - 1) = element;
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn pop(&mut self) {
        assert!(self.size > 0);
        self.size -= 1;
        // just swap 
        unsafe {
            let mut val: T = MaybeUninit::uninit().assume_init();
            swap(&mut val, self.get_mut_unchecked(self.size).deref_mut());
            drop(val);
        }
    }
}

impl<T: Default> Vec<T> {
    pub fn new_with_size(size: usize) -> Self {
        let mut capacity = 1;
        while capacity < size {
            capacity *= 2;
        }
        let mut res = Self {
            capacity,
            size,
            arr: Ptr::new(capacity),
        };
        for i in 0..size {
            unsafe {
                *res.arr.get_mut().add(i) = T::default();
            }
        }
        res
    }
}

impl<T> Default for Vec<T> {
    fn default() -> Self {
        Self::new()
    }
}
