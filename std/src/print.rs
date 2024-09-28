use core::fmt;

static mut PRINT: Option<&dyn Fn(fmt::Arguments)> = None;

pub fn init_print(print: &'static dyn Fn(fmt::Arguments)) {
    unsafe {
        PRINT = Some(print);
    }
}

pub fn print_raw(args: fmt::Arguments) {
    if unsafe { PRINT.is_none() } {
        return;
    }

    let f = unsafe { PRINT.as_ref().unwrap() };
    f(args);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::print_raw(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
