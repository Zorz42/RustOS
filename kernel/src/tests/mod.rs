#[allow(unused_imports)]
use A0_rand::*;
use kernel_test::all_tests;

use crate::{print, println};

mod A0_rand;
mod A1_utils;
mod A2_bitset;
mod A3_paging;

#[cfg(debug_assertions)]
pub fn test_runner() {
    use crate::print::{reset_print_color, set_print_color, TextColor};

    let tests = all_tests!();

    set_print_color(TextColor::LightCyan, TextColor::Black);
    println!("Running {} tests", tests.len());
    for (test, name) in tests {
        print!("testing {name} ... ");
        test();
        println!("[ok]");
    }

    reset_print_color();
}
