#![no_std]
#![allow(non_camel_case_types)]

mod boxed;
mod pointer;
mod rand;
mod serial;
mod string;
mod vector;
mod print;
mod spinlock;
mod mutable;
mod malloc;
mod bitset;

use core::fmt;
pub use malloc::{free, malloc};
pub use boxed::Box;
pub use derive;
pub use rand::Rng;
pub use serial::{deserialize, serialize, Serial};
pub use string::String;
pub use vector::Vec;
pub use print::{init_print, print_raw};
pub use spinlock::Lock;
pub use mutable::{Mutable, MutableToken};
pub use bitset::{BitSet, BitSetRaw, bitset_size_bytes};
pub use malloc::{HEAP_REGION_SIZE};
use crate::malloc::init_malloc;

static mut PAGE_ALLOCATOR: Option<&'static dyn Fn(*mut u8, bool)> = None;
static mut PAGE_DEALLOCATOR: Option<&'static dyn Fn(*mut u8)> = None;
static mut HEAP_ADDR: u64 = 0;

struct Writer {}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        debug_str(s);
        Ok(())
    }
}

pub fn debug_raw(args: fmt::Arguments) {
    let mut writer = Writer {};
    fmt::Write::write_fmt(&mut writer, args).unwrap();
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::debug_raw(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! debugln {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::debug!("{}\n", format_args!($($arg)*)));
}

fn write_out(c: char) {
    let addr = 0x10000000 as *mut u8;
    unsafe {
        while addr.add(5).read_volatile() & (1 << 5) == 0 {}
        addr.write_volatile(c as u8);
    }
}

pub fn debug_str(s: &str) {
    for c in s.chars() {
        write_out(c);
    }
}

fn allocate_page(page: *mut u8, ignore_if_exists: bool) {
    unsafe {
        if let Some(page_allocator) = PAGE_ALLOCATOR {
            page_allocator(page, ignore_if_exists);
        } else {
            unreachable!("Std library memory was not initialized!");
        }
    }
}

fn deallocate_page(page: *mut u8) {
    unsafe {
        if let Some(page_deallocator) = PAGE_DEALLOCATOR {
            page_deallocator(page);
        } else {
            unreachable!("Std library memory was not initialized!");
        }
    }
}

pub fn init_std_memory(page_allocator: &'static dyn Fn(*mut u8, bool), page_deallocator: &'static dyn Fn(*mut u8), heap_addr: u64) {
    unsafe {
        PAGE_ALLOCATOR = Some(page_allocator);
        PAGE_DEALLOCATOR = Some(page_deallocator);
        HEAP_ADDR = heap_addr;
    }
    init_malloc();
}
