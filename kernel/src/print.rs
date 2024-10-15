use core::fmt;
use core::fmt::Write;
use std::Mutable;
use crate::riscv::{get_core_id, interrupts_get};
use crate::text_renderer::{get_screen_height_chars, get_screen_width_chars, render_text_to_screen, scroll, set_char, TextColor};
use crate::timer::get_ticks;

struct Writer {
    x: usize,
    text_color: TextColor,
    background_color: TextColor,
}

impl Writer {
    const fn new() -> Self {
        Self {
            x: 0,
            text_color: TextColor::White,
            background_color: TextColor::Black,
        }
    }

    fn set_color(&mut self, text_color: TextColor, background_color: TextColor) {
        self.text_color = text_color;
        self.background_color = background_color;
    }

    fn new_line(&mut self) {
        self.x = 0;
        scroll();
    }

    fn write_byte(&mut self, c: u8) {
        let addr = 0x10000000 as *mut u8;
        unsafe {
            while addr.add(5).read_volatile() & (1 << 5) == 0 {}
            addr.write_volatile(c);
        }

        if c == b'\n' {
            self.new_line();
            return;
        }
        if c == b'\r' {
            self.x = 0;
            for x in 0..get_screen_width_chars() {
                set_char(x, get_screen_height_chars() - 1, b' ', self.text_color, self.background_color);
            }
            return;
        }
        set_char(self.x, get_screen_height_chars() - 1, c, self.text_color, self.background_color);
        self.x += 1;
        if self.x >= get_screen_width_chars() {
            self.new_line();
        }
    }

    fn move_cursor_back(&mut self) {
        if self.x != 0 {
            self.x -= 1;
        }
    }
}

static WRITER: Mutable<Writer> = Mutable::new(Writer::new());

pub fn init_print() {
    std::init_print(&_print);
}

static LAST_REFRESH: Mutable<u64> = Mutable::new(0);

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    let t = WRITER.borrow();
    WRITER.get_mut(&t).write_fmt(args).unwrap();
    WRITER.release(t);
    check_screen_refresh_for_print();
}

const PRINT_REFRESH_INTERVAL: u64 = 50;

pub fn check_screen_refresh_for_print() {
    if get_core_id() != 0 {
        return;
    }

    let t = LAST_REFRESH.borrow();
    if get_ticks() - *LAST_REFRESH.get(&t) > PRINT_REFRESH_INTERVAL && interrupts_get() {
        render_text_to_screen();
        *LAST_REFRESH.get_mut(&t) = get_ticks();
    }
    LAST_REFRESH.release(t);
}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            self.write_byte(c);
        }
        Ok(())
    }
}

pub fn set_print_color(text_color: TextColor, background_color: TextColor) {
    let t = WRITER.borrow();
    WRITER.get_mut(&t).set_color(text_color, background_color);
    WRITER.release(t);
}

pub fn reset_print_color() {
    let t = WRITER.borrow();
    WRITER.get_mut(&t).set_color(TextColor::White, TextColor::Black);
    WRITER.release(t);
}

pub fn move_cursor_back() {
    let t = WRITER.borrow();
    WRITER.get_mut(&t).move_cursor_back();
    WRITER.release(t);
}