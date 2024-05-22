use crate::Vec;

pub trait Serial {
    fn serialize(&mut self, vec: &mut Vec<u8>);
    fn deserialize(vec: &Vec<u8>, idx: &mut usize) -> Self;
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

            fn deserialize(vec: &Vec<u8>, idx: &mut usize) -> Self {
                let obj = Self::default();
                for i in 0..core::mem::size_of::<$T>() {
                    unsafe {
                        let ptr = core::ptr::from_ref(&obj) as *mut u8;
                        *ptr.add(i) = vec[*idx];
                        *idx += 1;
                    }
                }
                obj
            }
        }
    };
}

pub fn serialize<T: Serial>(obj: &mut T) -> Vec<u8> {
    let mut vec = Vec::new();
    obj.serialize(&mut vec);
    vec
}

pub fn deserialize<T: Serial>(data: &Vec<u8>) -> T {
    let mut idx = 0;
    T::deserialize(data, &mut idx)
}

implement_serial_direct!(u8);
implement_serial_direct!(i8);
implement_serial_direct!(u16);
implement_serial_direct!(i16);
implement_serial_direct!(u32);
implement_serial_direct!(i32);
implement_serial_direct!(u64);
implement_serial_direct!(i64);
implement_serial_direct!(f32);
implement_serial_direct!(f64);
implement_serial_direct!(isize);
implement_serial_direct!(usize);
implement_serial_direct!(char);
