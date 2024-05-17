pub struct BitSetRaw {
    data: *mut u64,
    size: usize,
    count0: usize,
}

impl BitSetRaw {
    pub const fn new_empty() -> BitSetRaw {
        BitSetRaw {
            data: 0 as *mut u64,
            size: 0,
            count0: 0,
        }
    }

    pub fn new(size: usize, addr: *mut u64) -> BitSetRaw {
        debug_assert_eq!(addr as u64 % 8, 0);
        let mut res = BitSetRaw { data: addr, size, count0: 0 };
        res.clear();
        res
    }

    /// Takes from memory, does not clear
    pub fn new_from(size: usize, addr: *mut u64) -> BitSetRaw {
        debug_assert_eq!(addr as u64 % 8, 0);
        let mut res = BitSetRaw { data: addr, size, count0: 0 };
        for i in 0..size {
            if !res.get(i) {
                res.count0 += 1;
            }
        }
        res
    }

    pub fn set(&mut self, index: usize, val: bool) {
        debug_assert!(index < self.size);

        let byte_index = index / 64;
        let bit_index = index % 64;
        self.count0 += !val as usize;
        self.count0 -= !self.get(index) as usize;
        unsafe {
            if val {
                *self.data.add(byte_index) |= 1 << bit_index;
            } else {
                *self.data.add(byte_index) &= !(1 << bit_index);
            }
        }
    }

    pub fn get(&self, index: usize) -> bool {
        debug_assert!(index < self.size);

        let byte_index = index / 64;
        let bit_index = index % 64;
        unsafe { (*self.data.add(byte_index) & (1 << bit_index)) != 0 }
    }

    pub fn get_size_bytes(&self) -> usize {
        self.size / 8
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    fn get_num_u64(&self) -> usize {
        (self.size + 63) / 64
    }

    pub fn get_zero_element(&self) -> Option<usize> {
        if self.count0 == 0 {
            return None;
        }
        for i in 0..self.get_num_u64() {
            let mut val = unsafe { *self.data.add(i) };
            if val != 0xFFFF_FFFF_FFFF_FFFF {
                for j in 0..64 {
                    if val & 1 == 0 {
                        return Some(i * 64 + j);
                    }
                    val >>= 1;
                }
                unreachable!();
            }
        }
        None
    }

    pub fn clear(&mut self) {
        for i in 0..self.get_num_u64() {
            unsafe {
                *self.data.add(i) = 0;
            }
        }
        self.count0 = self.size;
    }

    pub fn get_count0(&self) -> usize {
        self.count0
    }
}
