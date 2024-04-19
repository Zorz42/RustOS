use crate::{print, println, vga_driver};
use crate::print::{reset_print_color, set_print_color};

mod elementary;

pub trait Testable {
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
    set_print_color(vga_driver::VgaColor::LightCyan, vga_driver::VgaColor::Black);
    println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }

    reset_print_color();
}