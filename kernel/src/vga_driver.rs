use crate::font::{CHAR_HEIGHT, CHAR_WIDTH, DEFAULT_FONT};
use crate::memory::{memcpy, memset_int64, volatile_store_byte};

struct VgaBinding {
    width: usize,
    height: usize,
    stride: usize,
    bytes_per_pixel: usize,
    framebuffer: *mut u8,
}

static mut VGA_BINDING: VgaBinding = VgaBinding {
    width: 0,
    height: 0,
    stride: 0,
    bytes_per_pixel: 0,
    framebuffer: core::ptr::null_mut(),
};

pub fn init(
    width: usize,
    height: usize,
    stride: usize,
    bytes_per_pixel: usize,
    framebuffer: *mut u8,
) {
    unsafe {
        VGA_BINDING.width = width;
        VGA_BINDING.height = height;
        VGA_BINDING.stride = stride;
        VGA_BINDING.bytes_per_pixel = bytes_per_pixel;
        VGA_BINDING.framebuffer = framebuffer;
    }
}

const BORDER_PADDING: usize = 8;

pub fn get_screen_width_in_chars() -> usize {
    unsafe { (VGA_BINDING.width - 2 * BORDER_PADDING) / CHAR_WIDTH }
}

pub fn get_screen_height_in_chars() -> usize {
    unsafe { (VGA_BINDING.height - 2 * BORDER_PADDING) / CHAR_HEIGHT }
}

pub fn get_screen_width() -> usize {
    unsafe { VGA_BINDING.width }
}

pub fn get_screen_height() -> usize {
    unsafe { VGA_BINDING.height }
}

#[inline]
fn get_pixel_mut(x: usize, y: usize) -> *mut u8 {
    unsafe {
        debug_assert!(x < VGA_BINDING.width);
        debug_assert!(y < VGA_BINDING.height);
        let offset = (y * VGA_BINDING.stride + x) * VGA_BINDING.bytes_per_pixel;
        VGA_BINDING.framebuffer.add(offset)
    }
}

#[inline]
pub fn set_pixel(x: usize, y: usize, color: (u8, u8, u8)) {
    unsafe {
        let pixel_pointer = get_pixel_mut(x, y);
        volatile_store_byte(pixel_pointer, color.0);
        volatile_store_byte(pixel_pointer.add(1), color.1);
        volatile_store_byte(pixel_pointer.add(2), color.2);
    }
}

#[inline]
pub fn get_pixel(x: usize, y: usize) -> (u8, u8, u8) {
    unsafe {
        let pixel_pointer = get_pixel_mut(x, y);
        let r = *pixel_pointer;
        let g = *pixel_pointer.add(1);
        let b = *pixel_pointer.add(2);
        (r, g, b)
    }
}

pub fn set_char(
    x: usize,
    y: usize,
    c: u8,
    text_color: (u8, u8, u8),
    background_color: (u8, u8, u8),
) {
    debug_assert!(x < get_screen_width_in_chars());
    debug_assert!(y < get_screen_height_in_chars());

    let screen_x = x * CHAR_WIDTH + BORDER_PADDING;
    let screen_y = y * CHAR_HEIGHT + BORDER_PADDING;
    for char_y in 0..CHAR_HEIGHT {
        for char_x in 0..CHAR_WIDTH {
            let pixel_x = screen_x + char_x;
            let pixel_y = screen_y + char_y;
            let color = if DEFAULT_FONT[c as usize * CHAR_HEIGHT + char_y]
                & (1 << (CHAR_WIDTH - char_x - 1))
                != 0
            {
                text_color
            } else {
                background_color
            };
            set_pixel(pixel_x, pixel_y, color);
        }
    }
}

pub fn scroll() {
    unsafe {
        for y in BORDER_PADDING..VGA_BINDING.height - BORDER_PADDING - CHAR_HEIGHT {
            let src = VGA_BINDING
                .framebuffer
                .add((y + CHAR_HEIGHT) * VGA_BINDING.stride * VGA_BINDING.bytes_per_pixel);
            let dest = VGA_BINDING
                .framebuffer
                .add(y * VGA_BINDING.stride * VGA_BINDING.bytes_per_pixel);
            memcpy(src, dest, VGA_BINDING.width * VGA_BINDING.bytes_per_pixel);
        }

        for y in
            VGA_BINDING.height - BORDER_PADDING - CHAR_HEIGHT..VGA_BINDING.height - BORDER_PADDING
        {
            let dest = VGA_BINDING
                .framebuffer
                .add(y * VGA_BINDING.stride * VGA_BINDING.bytes_per_pixel);
            memset_int64(dest, 0, VGA_BINDING.width * VGA_BINDING.bytes_per_pixel);
        }
    }
}

pub fn clear_screen() {
    for y in 0..get_screen_height() {
        unsafe {
            let dest = VGA_BINDING
                .framebuffer
                .add(y * VGA_BINDING.stride * VGA_BINDING.bytes_per_pixel);
            memset_int64(dest, 0, VGA_BINDING.width * VGA_BINDING.bytes_per_pixel);
        }
    }
}
