use core::ops::{Deref, DerefMut};
use core::ptr::write_bytes;
use crate::Vec;

/// Layout of the bitset has size times u32.
/// 31st bit is the actual bit, 30th bit indicated if the bit is in the stack.
/// Integer from 0 to 29th bit is the value on stack.
/// Stack contain indexes of 0 bits so that we can find them in O(1) time.
pub struct BitSetRaw {
    data: *mut u64,
    size: usize,
    count0: usize,
    stack_size: usize,
}

pub unsafe fn get_raw(base: *const u64, index: usize) -> bool {
    let byte_index = index / 64;
    let bit_index = index % 64;
    (*base.add(byte_index) & (1 << bit_index)) != 0
}

pub unsafe fn set_raw(base: *mut u64, index: usize, val: bool) {
    let byte_index = index / 64;
    let bit_index = index % 64;

    let mut x = *base.add(byte_index);
    x |= 1 << bit_index;
    x ^= (1 << bit_index) * (!val) as u64;
    *base.add(byte_index) = x;
}

pub const fn bitset_size_bytes(size: usize) -> usize {
    size * 4
}

impl BitSetRaw {
    pub const fn new_empty() -> Self {
        Self {
            data: 0 as *mut u64,
            size: 0,
            count0: 0,
            stack_size: 0,
        }
    }

    pub fn new(size: usize, addr: *mut u64) -> Self {
        #[cfg(feature = "assertions")]
        assert_eq!(addr as u64 % 8, 0);
        let mut res = Self {
            data: addr,
            size,
            count0: 0,
            stack_size: 0,
        };
        res.clear();
        res
    }

    /// Takes from memory, does not clear
    pub fn new_from(size: usize, addr: *mut u64) -> Self {
        #[cfg(feature = "assertions")]
        assert_eq!(addr as u64 % 8, 0);
        let mut res = Self {
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
        for i in 0..self.size {
            unsafe {
                // disable is_on_stack bit
                set_raw(self.data, 32 * i + 30, false);
            }
        }

        for i in 0..self.size {
            if !self.get(i) {
                self.add_to_stack(i);
            }
        }

        #[cfg(feature = "assertions")]
        assert!(self.stack_size >= self.count0);
    }

    fn add_to_stack(&mut self, index: usize) {
        if unsafe { get_raw(self.data, 32 * index + 30) } {
            return;
        }

        unsafe {
            set_raw(self.data, 32 * index + 30, true);
        }
        #[cfg(feature = "assertions")]
        assert!(self.stack_size < self.size);

        unsafe {
            let addr = (self.data as *mut u32).add(self.stack_size);
            let mut data = addr.read();
            data &= (1 << 31) | (1 << 30);
            data |= index as u32;
            addr.write(data);
        }

        self.stack_size += 1;
    }

    fn stack_top(&mut self) -> usize {
        #[cfg(feature = "assertions")]
        assert!(self.stack_size >= self.count0);

        unsafe {
            let addr = (self.data as *mut u32).add(self.stack_size - 1);
            let mut data = addr.read();
            data &= !((1 << 31) | (1 << 30));
            data as usize
        }
    }

    fn pop_stack(&mut self) {
        #[cfg(feature = "assertions")]
        assert!(self.stack_size >= self.count0);

        unsafe {
            set_raw(self.data, 32 * self.stack_top() + 30, false);
        }

        self.stack_size -= 1;
    }

    pub fn set(&mut self, index: usize, val: bool) {
        #[cfg(feature = "assertions")]
        assert!(index < self.size);

        self.count0 += !val as usize;
        self.count0 -= !self.get(index) as usize;

        unsafe {
            set_raw(self.data, 32 * index + 31, val);
        }

        if !val {
            self.add_to_stack(index);
        }

        #[cfg(feature = "assertions")]
        assert!(self.stack_size >= self.count0);
    }

    pub fn get(&self, index: usize) -> bool {
        #[cfg(feature = "assertions")]
        assert!(index < self.size);
        unsafe {
            get_raw(self.data, 32 * index + 31)
        }
    }

    pub const fn get_size_bytes(&self) -> usize {
        bitset_size_bytes(self.size)
    }

    pub const fn get_size(&self) -> usize {
        self.size
    }

    pub fn get_zero_element(&mut self) -> Option<usize> {
        if self.count0 == 0 {
            return None;
        }

        loop {
            let idx = self.stack_top();
            if !self.get(idx) {
                return Some(idx);
            }
            self.pop_stack();
        }
    }

    pub fn clear(&mut self) {
        unsafe {
            write_bytes(self.data as *mut u8, 0, self.get_size_bytes());
        }
        self.count0 = self.size;
        self.setup_stack();
    }

    pub const fn get_count0(&self) -> usize {
        self.count0
    }

    pub unsafe fn load_from(&mut self, ptr: *mut u64) {
        for i in 0..self.size {
            let val = get_raw(ptr, i);
            set_raw(self.data, 32 * i + 31, val);
        }

        self.update_count0();
        self.setup_stack();
    }

    pub unsafe fn store_to(&self, ptr: *mut u64) {
        for i in 0..self.size {
            let val = get_raw(self.data, 32 * i + 31);
            set_raw(ptr, i, val);
        }
    }

    // adds one more element to the bitset
    pub fn add_one(&mut self) {
        self.size += 1;
        self.count0 += 1;
        unsafe {
            set_raw(self.data, 32 * (self.size - 1) + 30, false);
            set_raw(self.data, 32 * (self.size - 1) + 31, false);
            self.add_to_stack(self.size - 1);
        }
    }
}

#[allow(dead_code)] // data is not used but its pointer is
pub struct BitSet {
    bitset: BitSetRaw,
    data: Vec<u8>,
}

impl BitSet {
    pub fn new(size: usize) -> Self {
        let mut data = Vec::new_with_size(bitset_size_bytes(size));
        Self {
            bitset: BitSetRaw::new_from(size, data.as_mut_ptr() as *mut u64),
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
