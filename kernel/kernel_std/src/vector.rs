use core::fmt::{Debug, Display, Formatter};
use crate::pointer::Ptr;
use crate::serial::Serial;
use core::ops::{DerefMut, Index, IndexMut};
use core::ptr::copy_nonoverlapping;
use crate::debug_str;

pub struct Vec<T> {
    arr: Ptr<T>,
    size: usize,
    capacity: usize,
}

pub struct VecIntoIterator<T> {
    vec: Vec<T>,
    index: usize,
}

pub struct VecIterator<'a, T> {
    vec: &'a Vec<T>,
    index: usize,
}

pub struct VecMutIterator<'a, T> {
    vec: &'a mut Vec<T>,
    index: usize,
}

impl<T> Vec<T> {
    pub unsafe fn new_with_size_uninit(size: usize) -> Self {
        let mut capacity = 1;
        while capacity < size {
            capacity *= 2;
        }
        Self {
            capacity,
            size,
            arr: Ptr::new(capacity),
        }
    }

    pub const unsafe fn new_empty() -> Self {
        Self {
            capacity: 0,
            size: 0,
            arr: Ptr::new_empty(),
        }
    }

    pub fn new() -> Self {
        unsafe { Self::new_with_size_uninit(0) }
    }

    pub unsafe fn get_unchecked(&self, i: usize) -> &T {
        &*self.arr.get().add(i)
    }

    pub unsafe fn get_mut_unchecked(&mut self, i: usize) -> &mut T {
        &mut *self.arr.get_mut().add(i)
    }

    pub fn get(&self, i: usize) -> Option<&T> {
        if i < self.size {
            unsafe { Some(self.get_unchecked(i)) }
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        if i < self.size {
            unsafe { Some(self.get_mut_unchecked(i)) }
        } else {
            None
        }
    }

    pub fn reserve(&mut self, size: usize) {
        if size <= self.capacity {
            return;
        }
        let mut new_capacity = self.capacity;
        while new_capacity < size {
            new_capacity *= 2;
        }
        let mut new_arr = unsafe { Ptr::new(new_capacity) };
        unsafe {
            copy_nonoverlapping(self.arr.get_mut() as *mut u8, new_arr.get_mut() as *mut u8, (self.size * size_of::<T>()).div_ceil(8) * 8);
        }
        self.arr = new_arr;
        self.capacity = new_capacity;
    }

    pub fn push(&mut self, element: T) -> &mut T {
        self.reserve(self.size + 1);
        self.size += 1;
        unsafe {
            core::ptr::write(self.get_mut_unchecked(self.size - 1), element);
            self.get_mut_unchecked(self.size - 1)
        }
    }

    pub unsafe fn push_uninit(&mut self, num: usize) {
        self.reserve(self.size + num);
        self.size += num;
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.size == 0 {
            None
        } else {
            self.size -= 1;
            unsafe { Some(core::ptr::read(self.get_mut_unchecked(self.size).deref_mut())) }
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn reverse(&mut self) {
        let size = self.size;
        for i in 0..size / 2 {
            let ptr1 = &mut self[i] as *mut T;
            let ptr2 = &mut self[size - 1 - i] as *mut T;
            unsafe {
                core::mem::swap(&mut *ptr1, &mut *ptr2);
            }
        }
    }

    pub fn retain(&mut self, filter: &dyn Fn(&T) -> bool) {
        let mut new_vec = Vec::new();
        self.reverse();
        while let Some(el) = self.pop() {
            if filter(&el) {
                new_vec.push(el);
            }
        }
        *self = new_vec;
    }
    
    pub fn as_ptr(&self) -> *const T {
        self.arr.get()
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.arr.get_mut()
    }
    
    pub fn sort(&mut self, f: &dyn Fn(&T, &T) -> bool) {
        for t in 0..self.size {
            for i in 0..self.size - 1 - t {
                if !f(&self[i], &self[i + 1]) {
                    let ptr1 = &mut self[i] as *mut T;
                    let ptr2 = &mut self[i + 1] as *mut T;
                    core::mem::swap(unsafe { &mut *ptr1 }, unsafe { &mut *ptr2  });
                }
            }
        }
    }
}

impl<T: Default> Vec<T> {
    pub fn new_with_size(size: usize) -> Self {
        let mut res = unsafe { Self::new_with_size_uninit(size) };
        for i in 0..size {
            unsafe {
                *res.arr.get_mut().add(i) = T::default();
            }
        }
        res
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        while self.size() > 0 {
            self.pop();
        }
    }
}

impl<T> Default for Vec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Vec<T> {
    pub fn new_from_slice(slice: &[T]) -> Self {
        let mut res = Self::new();
        for i in slice {
            res.push(i.clone());
        }
        res
    }
}

impl<T> Index<usize> for Vec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<T> IndexMut<usize> for Vec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

impl<T> Iterator for VecIntoIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.vec.size() {
            None
        } else {
            let res = unsafe { core::ptr::read(self.vec.get_mut_unchecked(self.index)) };
            self.index += 1;
            Some(res)
        }
    }
}

impl<T> Drop for VecIntoIterator<T> {
    fn drop(&mut self) {
        // make sure to drop all remaining
        while self.next().is_some() {}
        self.vec.size = 0;
    }
}

impl<'a, T> Iterator for VecIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.vec.size() {
            None
        } else {
            let res = unsafe { self.vec.get_unchecked(self.index) };
            self.index += 1;
            Some(res)
        }
    }
}

impl<'a, T> Iterator for VecMutIterator<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.vec.size() {
            None
        } else {
            // I hope this is safe.
            let res = unsafe { self.vec.get_mut_unchecked(self.index) as *mut T };
            self.index += 1;
            Some(unsafe { &mut *res })
        }
    }
}

impl<T> IntoIterator for Vec<T> {
    type Item = T;
    type IntoIter = VecIntoIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        VecIntoIterator { vec: self, index: 0 }
    }
}

impl<'a, T> IntoIterator for &'a Vec<T> {
    type Item = &'a T;
    type IntoIter = VecIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        VecIterator { vec: self, index: 0 }
    }
}

impl<'a, T> IntoIterator for &'a mut Vec<T> {
    type Item = &'a mut T;
    type IntoIter = VecMutIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        VecMutIterator { vec: self, index: 0 }
    }
}

impl<T: PartialEq> PartialEq for Vec<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.size != other.size {
            return false;
        }

        for i in 0..self.size {
            if unsafe { self.get_unchecked(i) } != unsafe { other.get_unchecked(i) } {
                return false;
            }
        }

        true
    }
}

impl<T: Clone> Clone for Vec<T> {
    fn clone(&self) -> Self {
        let mut res: Vec<T> = Vec::new();
        res.reserve(self.size);
        for i in self {
            unsafe {
                let addr = res.arr.get_mut().add(res.size) as u64;
            }
            res.push(i.clone());
        }
        res
    }
}

impl<T: Serial> Serial for Vec<T> {
    fn serialize(&mut self, vec: &mut Vec<u8>) {
        self.size.serialize(vec);
        for i in self {
            i.serialize(vec);
        }
    }

    fn deserialize(vec: &Vec<u8>, idx: &mut usize) -> Self {
        let mut obj = Vec::new();
        let size = usize::deserialize(vec, idx);
        for _ in 0..size {
            obj.push(T::deserialize(vec, idx));
        }
        obj
    }
}

impl<T: Display> Display for Vec<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "[")?;
        for i in 0..self.size {
            write!(f, "{}", self[i])?;
            if i != self.size - 1 {
                write!(f, ", ")?;
            }
        }
        write!(f, "]")
    }
}

impl<T: Debug> Debug for Vec<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "Vec[")?;
        for i in 0..self.size {
            write!(f, "{:?}", self[i])?;
            if i != self.size - 1 {
                write!(f, ", ")?;
            }
        }
        write!(f, "]")
    }
}