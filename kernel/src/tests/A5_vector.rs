use crate::tests::KernelPerf;
use kernel_test::{kernel_perf, kernel_test, kernel_test_mod};
use kernel_std::{deserialize, serialize, Rng, Serial, Vec};
kernel_test_mod!(crate::tests::A5_vector);

#[kernel_test]
fn test_vector_new() {
    for _ in 0..1000 {
        let vec = Vec::<i32>::new();
        assert_eq!(vec.size(), 0);
    }
}

#[kernel_test]
fn test_vector_new_with_size() {
    for i in 0..1000 {
        let vec = Vec::<i32>::new_with_size(i);
        assert_eq!(vec.size(), i);
    }
}

#[kernel_test]
fn test_vector_push_and_index() {
    let mut arr = [0; 1024];
    let mut vec = Vec::new();

    let mut rng = Rng::new(543758924052);

    for i in 0..1024 {
        arr[i] = rng.get(0, 1 << 32) as u32;
        vec.push(arr[i]);

        for j in 0..i + 1 {
            assert_eq!(arr[j], vec[j]);
        }
    }
}

#[kernel_test]
fn test_vector_pop() {
    let mut arr = [0; 1024];
    let mut arr_size = 0;
    let mut vec = Vec::new();

    let mut rng = Rng::new(543758924052);

    for i in 0..1024 {
        if rng.get(0, 2) == 0 && vec.size() != 0 {
            vec.pop();
            arr_size -= 1;
        } else {
            arr[arr_size] = rng.get(0, 1 << 32) as u32;
            vec.push(arr[arr_size]);
            arr_size += 1
        }

        assert_eq!(arr_size, vec.size());
        for j in 0..vec.size() {
            assert_eq!(arr[j], vec[j]);
        }
    }
}

#[kernel_test]
fn test_vector_index_out_of_bounds() {
    let mut rng = Rng::new(62346234);
    let mut arr = [0; 1024];
    for _ in 0..100 {
        let size = rng.get(0, 1024) as usize;
        let mut vec = Vec::new();
        for i in 0..size {
            arr[i] = rng.get(0, 1u64 << 63);
            vec.push(arr[i]);
        }

        for _ in 0..10000 {
            let idx = rng.get(0, 5000) as usize;
            assert_eq!(vec.get(idx).is_none(), idx >= size);
        }
    }
}

static mut DROP_COUNTER: i32 = 0;

struct DroppableStruct {}

impl Drop for DroppableStruct {
    fn drop(&mut self) {
        unsafe {
            DROP_COUNTER += 1;
        }
    }
}

#[kernel_test]
fn test_vector_calls_drop_on_delete() {
    let mut rng = Rng::new(234567890987654);
    for _ in 0..1000 {
        let mut vec = Vec::new();
        let len = rng.get(0, 100);
        for _ in 0..len {
            vec.push(DroppableStruct {});
        }
        unsafe {
            assert_eq!(DROP_COUNTER, 0);
            DROP_COUNTER = 0;
        }
        drop(vec);
        unsafe {
            assert_eq!(DROP_COUNTER, len as i32);
            DROP_COUNTER = 0;
        }
    }
}

#[kernel_test]
fn test_vector_calls_drop_on_pop() {
    let mut rng = Rng::new(678543456378);
    let mut vec = Vec::new();
    for _ in 0..1000 {
        let len = rng.get(0, 100);
        for _ in 0..len {
            vec.push(DroppableStruct {});
        }
        unsafe {
            assert_eq!(DROP_COUNTER, 0);
            DROP_COUNTER = 0;
        }
        for _ in 0..len {
            vec.pop();
            unsafe {
                assert_eq!(DROP_COUNTER, 1);
                DROP_COUNTER = 0;
            }
        }
    }
}

#[kernel_test]
fn test_vector_partial_eq() {
    let mut rng = Rng::new(46378596243);

    for _ in 0..1000 {
        let mut vec1 = Vec::new();
        let len = rng.get(0, 1000) as usize;
        for _ in 0..len {
            vec1.push(rng.get(0, 1u64 << 63));
        }
        let mut vec2 = vec1.clone();
        assert!(vec1 == vec2);
        assert!(vec2 == vec1);
        let idx = rng.get(0, len as u64) as usize;
        // this is almost certainly not going to change the value
        vec1[idx] = rng.get(0, 1u64 << 63);
        assert!(vec1 != vec2);
        assert!(vec2 != vec1);
    }
}

fn test_serialize<T: Serial + TryFrom<u64> + PartialEq>()
where
    <T as TryFrom<u64>>::Error: core::fmt::Debug,
{
    let mut rng = Rng::new(57438295724389);
    for _ in 0..100 {
        let size = rng.get(0, 100) as usize;
        let mut vec1 = Vec::new();
        let mut bits = 0;
        for _ in 0..core::mem::size_of::<T>() {
            bits = 2 * bits + 1;
        }
        for _ in 0..size {
            vec1.push(T::try_from(rng.get(0, 1u64 << 63) & bits).unwrap());
        }
        let data = serialize(&mut vec1);
        let vec2 = deserialize(&data);

        assert!(vec1 == vec2);
    }
}

#[derive(kernel_std::derive::Serial, PartialEq)]
struct Sample {
    a: i32,
    b: u8,
    c: i32,
}

impl TryFrom<u64> for Sample {
    type Error = ();

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Ok(Self {
            a: ((value >> 0) & 0xFFFFFFFF) as i32,
            b: ((value >> 5) & 0xFF) as u8,
            c: ((value >> 32) & 0xFFFFFFFF) as i32,
        })
    }
}

#[kernel_test]
fn test_vector_serialize() {
    test_serialize::<i8>();
    test_serialize::<u8>();
    test_serialize::<i16>();
    test_serialize::<u16>();
    test_serialize::<i32>();
    test_serialize::<u32>();
    test_serialize::<i64>();
    test_serialize::<u64>();
    test_serialize::<isize>();
    test_serialize::<usize>();
    test_serialize::<Sample>();
}

#[kernel_test]
fn test_vector_drop_on_iter() {
    unsafe {
        assert_eq!(DROP_COUNTER, 0);
        DROP_COUNTER = 0;
    }
    {
        let mut vec = Vec::new();
        for _ in 0..4 {
            vec.push(DroppableStruct {});
        }

        let mut iter = vec.into_iter();
        assert!(iter.next().is_some());
        assert!(iter.next().is_some());
        unsafe {
            assert_eq!(DROP_COUNTER, 2);
        }
    }
    unsafe {
        assert_eq!(DROP_COUNTER, 4);
        DROP_COUNTER = 0;
    }
}

#[kernel_test]
fn test_vector_reverse() {
    let mut rng = Rng::new(56743852);

    for _ in 0..100 {
        let size = rng.get(0, 1000) as usize;
        let mut vec1 = Vec::new();

        for _ in 0..size {
            vec1.push(rng.get(0, 1u64 << 63));
        }

        let mut vec2 = vec1.clone();
        vec2.reverse();

        for i in 0..size {
            assert_eq!(vec1[i], vec2[size - i - 1]);
        }
    }
}

#[kernel_perf]
struct PerfVecPush10 {
    rng: Rng,
}

impl KernelPerf for PerfVecPush10 {
    fn setup() -> Self {
        Self { rng: Rng::new(564378254) }
    }

    fn run(&mut self) {
        let mut vec = Vec::new();
        for _ in 0..10 {
            vec.push(self.rng.get(0, 1u64 << 63));
        }
    }
}

#[kernel_perf]
struct PerfVecPush1000 {
    rng: Rng,
}

impl KernelPerf for PerfVecPush1000 {
    fn setup() -> Self {
        Self { rng: Rng::new(65347852643) }
    }

    fn run(&mut self) {
        let mut vec = Vec::new();
        for _ in 0..10000 {
            vec.push(self.rng.get(0, 1u64 << 63));
        }
    }
}
