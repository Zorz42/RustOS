#![no_std]
#![no_main]

use std::{get_pid, get_ticks, println};

#[std::std_main]
fn main() {
    println!("Hello, world!");

    let mut curr_ticks = get_ticks() / 1000;
    loop {
        if get_ticks() / 1000 != curr_ticks {
            curr_ticks = get_ticks() / 1000;
            println!("Ticks {}: {}", get_pid(), get_ticks());
            if curr_ticks >= 5 + get_pid() {
                break;
            }
        }
    }
}