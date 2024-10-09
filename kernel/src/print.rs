use core::fmt;
use core::intrinsics::{copy_nonoverlapping, write_bytes};
use crate::font::{CHAR_HEIGHT, CHAR_WIDTH, DEFAULT_FONT};
use crate::gpu::{get_framebuffer, get_screen_size, refresh_screen};
use std::Lock;
use core::fmt::Write;
use std::Mutable;
use crate::riscv::interrupts_get;
use crate::timer::get_ticks;

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

const fn text_color_to_rgb(color: TextColor) -> (u8, u8, u8) {
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

const BORDER_PADDING: usize = 8;

pub fn scroll() {
    unsafe {
        for y in BORDER_PADDING..get_screen_size().1 as usize - BORDER_PADDING - CHAR_HEIGHT {
            let src = get_framebuffer().add((y + CHAR_HEIGHT) * get_screen_size().0 as usize);
            let dest = get_framebuffer().add(y * get_screen_size().0 as usize);
            copy_nonoverlapping(src, dest, get_screen_size().0 as usize);
        }

        for y in get_screen_size().1 as usize - BORDER_PADDING - CHAR_HEIGHT..get_screen_size().1 as usize - BORDER_PADDING {
            let dest = get_framebuffer().add(y * get_screen_size().0 as usize);
            write_bytes(dest, 0, get_screen_size().0 as usize);
        }
    }
}

fn clear_screen() {
    for y in 0..get_screen_size().1 as usize {
        unsafe {
            let dest = get_framebuffer().add(y * get_screen_size().0 as usize);
            write_bytes(dest, 0, get_screen_size().0 as usize);
        }
    }
}

fn get_pixel_mut(x: usize, y: usize) -> *mut u32 {
    unsafe {
        debug_assert!(x < get_screen_size().0 as usize);
        debug_assert!(y < get_screen_size().1 as usize);
        let offset = y * get_screen_size().0 as usize + x;
        get_framebuffer().add(offset)
    }
}

fn set_pixel(x: usize, y: usize, color: (u8, u8, u8)) {
    unsafe {
        let pixel_pointer = get_pixel_mut(x, y);
        *pixel_pointer = ((color.0 as u32) << 16) | ((color.1 as u32) << 8) | color.2 as u32;
    }
}

pub fn set_char(x: usize, y: usize, c: u8, text_color: (u8, u8, u8), background_color: (u8, u8, u8)) {
    let width_chars = (get_screen_size().0 as usize - 2 * BORDER_PADDING) / CHAR_HEIGHT;
    let height_chars = (get_screen_size().1 as usize - 2 * BORDER_PADDING) / CHAR_HEIGHT;
    debug_assert!(x < width_chars);
    debug_assert!(y < height_chars);

    let screen_x = x * CHAR_WIDTH + BORDER_PADDING;
    let screen_y = y * CHAR_HEIGHT + BORDER_PADDING;
    for char_y in 0..CHAR_HEIGHT {
        for char_x in 0..CHAR_WIDTH {
            let pixel_x = screen_x + char_x;
            let pixel_y = screen_y + char_y;
            let color = if DEFAULT_FONT[c as usize * CHAR_HEIGHT + char_y] & (1 << (CHAR_WIDTH - char_x - 1)) != 0 {
                text_color
            } else {
                background_color
            };
            set_pixel(pixel_x, pixel_y, color);
        }
    }
}

struct Writer {
    x: usize,
    text_color: (u8, u8, u8),
    background_color: (u8, u8, u8),
}

impl Writer {
    const fn new() -> Self {
        Self {
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
        scroll();
    }

    fn write_byte(&mut self, c: u8) {
        let addr = 0x10000000 as *mut u8;
        unsafe {
            while addr.add(5).read_volatile() & (1 << 5) == 0 {}
            addr.write_volatile(c);
        }

        let width_chars = (get_screen_size().0 as usize - 2 * BORDER_PADDING) / CHAR_HEIGHT;
        let height_chars = (get_screen_size().1 as usize - 2 * BORDER_PADDING) / CHAR_HEIGHT;

        if c == b'\n' {
            self.new_line();
            return;
        }
        if c == b'\r' {
            self.x = 0;
            for x in 0..width_chars {
                set_char(x, height_chars - 1, b' ', self.text_color, self.background_color);
            }
            return;
        }
        set_char(self.x, height_chars - 1, c, self.text_color, self.background_color);
        self.x += 1;
        if self.x >= width_chars {
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
    let t = LAST_REFRESH.borrow();
    if get_ticks() - *LAST_REFRESH.get(&t) > PRINT_REFRESH_INTERVAL && interrupts_get() {
        refresh_screen();
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