use crate::Vec;

pub trait Serial {
    fn serialize(&mut self, vec: &mut Vec<u8>);
    fn deserialize(&mut self, vec: &Vec<u8>, idx: &mut usize);
}

macro_rules! implement_serial_direct {
    ($T:ident) => {
        impl Serial for $T {
            fn serialize(&mut self, vec: &mut Vec<u8>) {
                for i in 0..core::mem::size_of::<$T>() {
                    unsafe {
                        let ptr = core::ptr::from_ref(self) as *const u8;
                        vec.push(*ptr.add(i));
                    }
                }
            }
        
            fn deserialize(&mut self, vec: &Vec<u8>, idx: &mut usize) {
                for i in 0..core::mem::size_of::<$T>() {
                    unsafe {
                        let ptr = core::ptr::from_ref(self) as *mut u8;
                        *ptr.add(i) = vec[*idx];
                        *idx += 1;
                    }
                }
            }
        }
    };
}

implement_serial_direct!(u8);
implement_serial_direct!(i8);
implement_serial_direct!(u16);
implement_serial_direct!(i16);
implement_serial_direct!(u32);
implement_serial_direct!(i32);
implement_serial_direct!(u64);
implement_serial_direct!(i64);
implement_serial_direct!(isize);
implement_serial_direct!(usize);