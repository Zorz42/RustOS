use core::intrinsics::write_bytes;
use std::{Mutable, Vec};
use crate::font::{CHAR_HEIGHT, CHAR_WIDTH, DEFAULT_FONT};
use crate::gpu::{get_framebuffer, get_screen_size, refresh_screen};

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

static SCREEN_CHARS: Mutable<Vec<(u8, TextColor, TextColor)>> = Mutable::new(unsafe { Vec::new_empty() });
static mut SCREEN_WIDTH_CHARS: usize = 0;
static mut SCREEN_HEIGHT_CHARS: usize = 0;

pub fn init_text_renderer() {
    unsafe {
        SCREEN_WIDTH_CHARS = (get_screen_size().0 as usize - 2 * BORDER_PADDING) / CHAR_HEIGHT;
        SCREEN_HEIGHT_CHARS = (get_screen_size().1 as usize - 2 * BORDER_PADDING) / CHAR_HEIGHT;
    }
    let t = SCREEN_CHARS.borrow();
    *SCREEN_CHARS.get_mut(&t) = unsafe { Vec::new_with_size_uninit(get_screen_width_chars() * get_screen_height_chars()) };
    for c in SCREEN_CHARS.get_mut(&t) {
        *c = (b' ', TextColor::White, TextColor::Black);
    }
    SCREEN_CHARS.release(t);
}

pub fn get_screen_width_chars() -> usize {
    unsafe {
        SCREEN_WIDTH_CHARS
    }
}

pub fn get_screen_height_chars() -> usize {
    unsafe {
        SCREEN_HEIGHT_CHARS
    }
}

pub fn scroll() {
    let t = SCREEN_CHARS.borrow();
    let screen_width = unsafe { SCREEN_WIDTH_CHARS };
    let screen_height = unsafe { SCREEN_HEIGHT_CHARS };
    let mut screen_chars = SCREEN_CHARS.get_mut(&t);
    for y in 1..screen_height {
        for x in 0..screen_width {
            screen_chars[(y - 1) * screen_width + x] = screen_chars[y * screen_width + x];
        }
    }
    for x in 0..screen_width {
        screen_chars[(screen_height - 1) * screen_width + x] = (b' ', TextColor::White, TextColor::Black);
    }
    SCREEN_CHARS.release(t);
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



fn draw_char(x: usize, y: usize, c: u8, text_color: (u8, u8, u8), background_color: (u8, u8, u8)) {
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

pub fn set_char(x: usize, y: usize, c: u8, text_color: TextColor, background_color: TextColor) {
    let t = SCREEN_CHARS.borrow();
    let screen_width = unsafe { SCREEN_WIDTH_CHARS };
    SCREEN_CHARS.get_mut(&t)[x + screen_width * y] = (c, text_color, background_color);
    SCREEN_CHARS.release(t);
}

pub fn render_text_to_screen() {
    let t = SCREEN_CHARS.borrow();
    let screen_chars = SCREEN_CHARS.get(&t).clone();
    SCREEN_CHARS.release(t);

    for y in 0..get_screen_height_chars() {
        for x in 0..get_screen_width_chars() {
            let ch = screen_chars[x + get_screen_width_chars() * y];
            draw_char(x, y, ch.0, text_color_to_rgb(ch.1), text_color_to_rgb(ch.2));
        }
    }

    refresh_screen();
}
