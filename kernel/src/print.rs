use crate::vga_driver;
use crate::vga_driver::{get_screen_height_in_chars, get_screen_width_in_chars};
use core::fmt;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextColor {
    Black,
    Blue,
    Green,
    Cyan,
    Red,
    Magenta,
    Brown,
    LightGray,
    DarkGray,
    LightBlue,
    LightGreen,
    LightCyan,
    LightRed,
    Pink,
    Yellow,
    White,
}

fn text_color_to_rgb(color: TextColor) -> (u8, u8, u8) {
    match color {
        TextColor::Black => (0, 0, 0),
        TextColor::Blue => (0, 0, 170),
        TextColor::Green => (0, 170, 0),
        TextColor::Cyan => (0, 170, 170),
        TextColor::Red => (170, 0, 0),
        TextColor::Magenta => (170, 0, 170),
        TextColor::Brown => (170, 85, 0),
        TextColor::LightGray => (170, 170, 170),
        TextColor::DarkGray => (85, 85, 85),
        TextColor::LightBlue => (85, 85, 255),
        TextColor::LightGreen => (85, 255, 85),
        TextColor::LightCyan => (85, 255, 255),
        TextColor::LightRed => (255, 85, 85),
        TextColor::Pink => (255, 85, 255),
        TextColor::Yellow => (255, 255, 85),
        TextColor::White => (255, 255, 255),
    }
}

struct Writer {
    x: usize,
    text_color: (u8, u8, u8),
    background_color: (u8, u8, u8),
}

impl Writer {
    const fn new() -> Writer {
        Writer {
            x: 0,
            text_color: (255, 255, 255),
            background_color: (0, 0, 0),
        }
    }

    fn set_color(&mut self, text_color: TextColor, background_color: TextColor) {
        self.text_color = text_color_to_rgb(text_color);
        self.background_color = text_color_to_rgb(background_color);
    }

    fn new_line(&mut self) {
        self.x = 0;
        vga_driver::scroll();
    }

    fn write_char(&mut self, c: u8) {
        if c == b'\n' {
            self.new_line();
            return;
        }
        if c == b'\r' {
            self.x = 0;
            return;
        }
        vga_driver::set_char(self.x, get_screen_height_in_chars() - 1, c, self.text_color, self.background_color);
        self.x += 1;
        if self.x >= get_screen_width_in_chars() {
            self.new_line();
        }
    }
}

static mut WRITER: Writer = Writer::new();

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::print::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    unsafe {
        WRITER.write_fmt(args).unwrap();
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            self.write_char(c);
        }
        Ok(())
    }
}

pub fn set_print_color(text_color: TextColor, background_color: TextColor) {
    unsafe {
        WRITER.set_color(text_color, background_color);
    }
}

pub fn reset_print_color() {
    unsafe {
        WRITER.set_color(TextColor::White, TextColor::Black);
    }
}
