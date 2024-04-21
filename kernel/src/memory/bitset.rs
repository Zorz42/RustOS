pub struct BitSetRaw {
    data: *mut u64,
    size: usize,
    count0: usize,
}

impl BitSetRaw {
    pub const fn new(size: usize, addr: *mut u64) -> BitSetRaw {
        BitSetRaw {
            data: addr,
            size: (size + 63) / 64 * 64,
            count0: 0,
        }
    }

    pub fn set(&mut self, index: usize, val: bool) {
        debug_assert!(index < self.size);

        let byte_index = index / 64;
        let bit_index = index % 64;
        self.count0 += val as usize ^ 1;
        self.count0 -= self.get(index) as usize ^ 1;
        unsafe {
            if val {
                *self.data.offset(byte_index as isize) |= 1 << bit_index;
            } else {
                *self.data.offset(byte_index as isize) &= !(1 << bit_index);
            }
        }
    }

    pub fn get(&self, index: usize) -> bool {
        debug_assert!(index < self.size);

        let byte_index = index / 64;
        let bit_index = index % 64;
        unsafe {
            (*self.data.offset(byte_index as isize) & (1 << bit_index)) != 0
        }
    }

    pub fn get_size_bytes(&self) -> usize {
        self.size / 8
    }
    
    pub fn get_size_bits(&self) -> usize { self.size }

    pub fn get_first_zero(&self) -> Option<usize> {
        for i in 0..self.size / 64 {
            let mut val = unsafe { *self.data.offset(i as isize) };
            if val != 0xFFFFFFFF_FFFFFFFF {
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
                *self.data.offset(i as isize) = 0;
            }
        }
        self.count0 = self.size;
    }
    
    pub fn get_count0(&mut self) -> usize {
        self.count0
    }
}