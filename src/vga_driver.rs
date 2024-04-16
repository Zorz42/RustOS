use core::intrinsics::volatile_set_memory;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum VgaColor {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

const VGA_BUFFER: *mut u8 = 0xb8000 as *mut u8;
pub const BUFFER_HEIGHT: usize = 25;
pub const BUFFER_WIDTH: usize = 80;

pub fn set_char(x: usize, y: usize, c: u8, text_color: VgaColor, background_color: VgaColor) {
    debug_assert!(x < BUFFER_WIDTH);
    debug_assert!(y < BUFFER_HEIGHT);
    
    let offset = y * BUFFER_WIDTH + x;
    unsafe {
        let char_pointer = VGA_BUFFER.offset((offset * 2) as isize);
        volatile_set_memory(char_pointer, c, 1);
        let color_pointer = VGA_BUFFER.offset((offset * 2 + 1) as isize);
        let color_val = (background_color as u8) << 4 | (text_color as u8);
        volatile_set_memory(color_pointer, color_val, 1);
    }
}

pub fn scroll() {
    for y in 1..BUFFER_HEIGHT {
        for x in 0..BUFFER_WIDTH {
            let offset = y * BUFFER_WIDTH + x;
            unsafe {
                let val1 = *VGA_BUFFER.offset(2 * offset as isize);
                let val2 = *VGA_BUFFER.offset(2 * offset as isize + 1);
                let prev_pointer1 = VGA_BUFFER.offset(2 * (offset - BUFFER_WIDTH) as isize);
                let prev_pointer2 = VGA_BUFFER.offset(2 * (offset - BUFFER_WIDTH) as isize + 1);
                volatile_set_memory(prev_pointer1, val1, 1);
                volatile_set_memory(prev_pointer2, val2, 1);
            }
        }
    }
    for x in 0..BUFFER_WIDTH {
        set_char(x, BUFFER_HEIGHT - 1, b' ', VgaColor::White, VgaColor::Black);
    }
}