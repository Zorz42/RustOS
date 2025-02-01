#![no_std]
#![no_main]

use std::println;

#[std::std_main]
fn main() {
    for i in 0..100 {
        sleep(1);
    }
}