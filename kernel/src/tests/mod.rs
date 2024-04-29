use crate::timer::get_ticks;

mod A0_rand;
mod A1_utils;
mod A2_bitset;
mod A3_paging;
mod A4_heap_tree;
mod A5_malloc;

#[cfg(feature = "run_tests")]
static mut FREE_SPACE: [u8; 1032] = [0; 1032];

#[cfg(feature = "run_tests")]
pub(super) fn get_free_space_addr() -> *mut u8 {
    unsafe { (FREE_SPACE.as_mut_ptr() as u64 / 8 * 8) as *mut u8 }
}

#[cfg(feature = "run_tests")]
pub fn test_runner() {
    use kernel_test::all_tests;

    use crate::print::{reset_print_color, set_print_color, TextColor};
    use crate::{print, println};

    let tests = all_tests!();

    let mut max_length = 0;

    for (_, name) in tests {
        max_length = max_length.max((name.len() + 9) / 10 * 10);
    }

    set_print_color(TextColor::Pink, TextColor::Black);
    println!("Running {} tests", tests.len());
    for (test, name) in tests {
        set_print_color(TextColor::LightCyan, TextColor::Black);
        print!("Testing {name}");
        let start_time = get_ticks();
        test();
        let end_time = get_ticks();
        set_print_color(TextColor::LightGreen, TextColor::Black);
        let width = max_length - name.len();
        for _ in 0..width {
            print!(" ");
        }
        print!("[OK] ");
        set_print_color(TextColor::LightGray, TextColor::Black);
        println!("{}ms", end_time - start_time);
    }

    reset_print_color();
}
