use kernel_test::kernel_test;

mod elementary;
mod memory;

/*pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("{} ... ", core::any::type_name::<T>());
        self();
        println!("[ok]");
    }
}

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Testable]) {
    use crate::print::{reset_print_color, set_print_color, TextColor};

    set_print_color(TextColor::LightCyan, TextColor::Black);
    println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }

    reset_print_color();
}*/

#[kernel_test]
fn test_example() {
    assert_eq!(1, 1);
}
