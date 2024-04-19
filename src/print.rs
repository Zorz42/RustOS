use core::fmt;
use crate::vga_driver;

struct Writer {
    x: usize,
    text_color: vga_driver::VgaColor,
    background_color: vga_driver::VgaColor,
}

impl Writer {
    const fn new() -> Writer {
        Writer {
            x: 0,
            text_color: vga_driver::VgaColor::White,
            background_color: vga_driver::VgaColor::Black,
        }
    }

    fn set_color(&mut self, text_color: vga_driver::VgaColor, background_color: vga_driver::VgaColor) {
        self.text_color = text_color;
        self.background_color = background_color;
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
        vga_driver::set_char(self.x, vga_driver::BUFFER_HEIGHT - 1, c, self.text_color, self.background_color);
        self.x += 1;
        if self.x >= vga_driver::BUFFER_WIDTH {
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

pub fn set_print_color(text_color: vga_driver::VgaColor, background_color: vga_driver::VgaColor) {
    unsafe {
        WRITER.set_color(text_color, background_color);
    }
}

pub fn reset_print_color() {
    unsafe {
        WRITER.set_color(vga_driver::VgaColor::White, vga_driver::VgaColor::Black);
    }
}