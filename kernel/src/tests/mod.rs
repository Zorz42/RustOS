mod A0_rand;
mod A1_utils;
mod A2_bitset;
mod A3_paging;

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

    set_print_color(TextColor::LightCyan, TextColor::Black);
    println!("Running {} tests", tests.len());
    for (test, name) in tests {
        print!("testing {name} ... ");
        test();
        println!("[ok]");
    }

    reset_print_color();
}
