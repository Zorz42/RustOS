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
        let mut res = BitSetRaw {
            data: addr,
            size: (size + 63) / 64 * 64,
            count0: 0,
        };
        res.clear();
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

    pub fn get_zero_element(&self) -> Option<usize> {
        for i in 0..self.size / 64 {
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
        for i in 0..self.size / 64 {
            unsafe {
                *self.data.add(i) = 0;
            }
        }
        self.count0 = self.size;
    }

    pub fn get_count0(&mut self) -> usize {
        self.count0
    }
}
