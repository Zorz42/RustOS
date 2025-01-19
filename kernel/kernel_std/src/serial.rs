use crate::Vec;
use core::ptr::copy_nonoverlapping;

pub trait Serial {
    fn serialize(&mut self, vec: &mut Vec<u8>);
    fn deserialize(vec: &Vec<u8>, idx: &mut usize) -> Self;
}

macro_rules! implement_serial_direct {
    ($T:ident) => {
        impl Serial for $T {
            fn serialize(&mut self, vec: &mut Vec<u8>) {
                unsafe {
                    let size = core::mem::size_of::<$T>();
                    vec.push_uninit(size);
                    let ptr = core::ptr::from_ref(self) as *const u8;
                    copy_nonoverlapping(ptr, vec.as_mut_ptr().add(vec.size() - size), size);
                }

            }

            fn deserialize(vec: &Vec<u8>, idx: &mut usize) -> Self {
                unsafe {
                    let ptr = vec.as_ptr().add(*idx) as *const $T;
                    *idx += core::mem::size_of::<$T>();
                    *ptr
                }
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

// implement serial for tuple
macro_rules! implement_serial_tuple {
    ($($T:ident),*) => {
        impl<$($T: Serial),*> Serial for ($($T,)*) {
            fn serialize(&mut self, vec: &mut Vec<u8>) {
                let ($(ref mut $T,)*) = *self;
                $($T.serialize(vec);)*
            }

            fn deserialize(vec: &Vec<u8>, idx: &mut usize) -> Self {
                ($($T::deserialize(vec, idx),)*)
            }
        }
    };
}

implement_serial_tuple!(a, b);
implement_serial_tuple!(a, b, c);
implement_serial_tuple!(a, b, c, d);
implement_serial_tuple!(a, b, c, d, e);
implement_serial_tuple!(a, b, c, d, e, f);
