use core::ops::{Deref, DerefMut};
use std::{memcpy, memset_int64, Vec};
use crate::println;

pub struct BitSetRaw {
    data: *mut u64,
    size: usize,
    count0: usize,
    stack_size: usize,
}

unsafe fn get_raw(base: *const u64, index: usize) -> bool {
    let byte_index = index / 64;
    let bit_index = index % 64;
    (*base.add(byte_index) & (1 << bit_index)) != 0
}

unsafe fn set_raw(base: *mut u64, index: usize, val: bool) {
    let byte_index = index / 64;
    let bit_index = index % 64;
    if val {
        *base.add(byte_index) |= 1 << bit_index;
    } else {
        *base.add(byte_index) &= !(1 << bit_index);
    }
}

pub const fn bitset_size_bytes(size: usize) -> usize {
    let s1 = (size + 63) / 64 * 8; // for actual bits
    let s2 = (size + 63) / 64 * 8; // for bits "is on stack"
    let s3 = size * 4; // for the stack
    return s1 + s2 + s3;
}

impl BitSetRaw {
    pub const fn new_empty() -> BitSetRaw {
        BitSetRaw {
            data: 0 as *mut u64,
            size: 0,
            count0: 0,
            stack_size: 0,
        }
    }

    pub fn new(size: usize, addr: *mut u64) -> BitSetRaw {
        debug_assert_eq!(addr as u64 % 8, 0);
        let mut res = BitSetRaw { 
            data: addr, 
            size, 
            count0: 0,
            stack_size: 0,
        };
        res.clear();
        res
    }

    /// Takes from memory, does not clear
    pub fn new_from(size: usize, addr: *mut u64) -> BitSetRaw {
        debug_assert_eq!(addr as u64 % 8, 0);
        let mut res = BitSetRaw {
            data: addr,
            size,
            count0: 0,
            stack_size: 0,
        };
        res.update_count0();
        res.setup_stack();
        res
    }
    
    fn update_count0(&mut self) {
        self.count0 = 0;
        for i in 0..self.size {
            if !self.get(i) {
                self.count0 += 1;
            }
        }
    }
    
    fn setup_stack(&mut self) {
        self.stack_size = 0;
        unsafe {
            memset_int64(self.get_stack_bitset_addr() as *mut u8, 0, self.get_num_u64() * 8);
        }
        for i in 0..self.size {
            if !self.get(i) {
                self.add_to_stack(i);
            }
        }
        assert!(self.stack_size >= self.count0);
    }
    
    fn get_stack_bitset_addr(&mut self) -> *mut u64 {
        unsafe {
            self.data.add(self.get_num_u64())
        }
    }

    fn get_stack_addr(&mut self) -> *mut i32 {
        unsafe {
            self.data.add(2 * self.get_num_u64()) as *mut i32
        }
    }
    
    fn add_to_stack(&mut self, index: usize) {
        if unsafe { get_raw(self.get_stack_bitset_addr(), index) } {
            return;
        }
        
        unsafe {
            set_raw(self.get_stack_bitset_addr(), index, true);
        }
        debug_assert!(self.stack_size < self.size);
        
        unsafe {
            *self.get_stack_addr().add(self.stack_size) = index as i32;
        }
        
        self.stack_size += 1;
    }
    
    fn stack_top(&mut self) -> i32 {
        assert!(self.stack_size >= self.count0);

        unsafe {
            *self.get_stack_addr().add(self.stack_size - 1)
        }
    }

    fn pop_stack(&mut self) {
        assert!(self.stack_size >= self.count0);
        
        unsafe {
            set_raw(self.get_stack_bitset_addr(), self.stack_top() as usize, false);
        }

        self.stack_size -= 1;
    }

    pub fn set(&mut self, index: usize, val: bool) {
        debug_assert!(index < self.size);
        
        self.count0 += !val as usize;
        self.count0 -= !self.get(index) as usize;

        unsafe {
            set_raw(self.data, index, val);
        }

        if !val {
            self.add_to_stack(index);
        }
        
        assert!(self.stack_size >= self.count0);
    }

    pub fn get(&self, index: usize) -> bool {
        debug_assert!(index < self.size);
        unsafe { 
            get_raw(self.data, index)
        }
    }

    pub fn get_size_bytes(&self) -> usize {
        bitset_size_bytes(self.size)
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    fn get_num_u64(&self) -> usize {
        (self.size + 63) / 64
    }

    pub fn get_zero_element(&mut self) -> Option<usize> {
        if self.count0 == 0 {
            return None;
        }
        
        loop {
            let idx = self.stack_top() as usize;
            if !self.get(idx) {
                return Some(idx);
            }
            self.pop_stack();
        }
    }

    pub fn clear(&mut self) {
        unsafe {
            memset_int64(self.data as *mut u8, 0, self.get_num_u64() * 8);
        }
        self.count0 = self.size;
        self.setup_stack();
    }

    pub fn get_count0(&self) -> usize {
        self.count0
    }
    
    pub unsafe fn load_from(&mut self, ptr: *mut u64) {
        memcpy(ptr as *mut u8, self.data as *mut u8, self.get_num_u64() * 8);
        self.update_count0();
        self.setup_stack();
    }
    
    pub unsafe fn store_to(&self, ptr: *mut u64) {
        memcpy(self.data as *mut u8, ptr as *mut u8, self.get_num_u64() * 8);
    }
}

pub struct BitSet {
    bitset: BitSetRaw,
    data: Vec<u8>,
}

impl BitSet {
    pub fn new(size: usize) -> Self {
        let data = Vec::new_with_size(bitset_size_bytes(size));
        Self {
            bitset: BitSetRaw::new_from(size, &data[0] as *const u8 as *mut u64),
            data,
        }
    }
}

impl Deref for BitSet {
    type Target = BitSetRaw;

    fn deref(&self) -> &Self::Target {
        &self.bitset
    }
}

impl DerefMut for BitSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bitset
    }
}