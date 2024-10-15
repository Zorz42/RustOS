use core::fmt;
use crate::Mutable;

static PRINT: Mutable<Option<&dyn Fn(fmt::Arguments)>> = Mutable::new(None);

pub fn init_print(print: &'static dyn Fn(fmt::Arguments)) {
    let t = PRINT.borrow();
    *PRINT.get_mut(&t) = Some(print);
    PRINT.release(t);
}

pub fn print_raw(args: fmt::Arguments) {
    let t = PRINT.borrow();
    if PRINT.get(&t).is_none() {
        PRINT.release(t);
        return;
    }

    let f = PRINT.get(&t).unwrap();
    f(args);
    PRINT.release(t);
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
