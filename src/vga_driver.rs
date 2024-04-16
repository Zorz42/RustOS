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
        *char_pointer = c;
        let color_pointer = VGA_BUFFER.offset((offset * 2 + 1) as isize);
        *color_pointer = (background_color as u8) << 4 | (text_color as u8);
    }
}

pub fn scroll() {
    let buffer = VGA_BUFFER as *mut u16;
    for y in 1..BUFFER_HEIGHT {
        for x in 0..BUFFER_WIDTH {
            let offset = y * BUFFER_WIDTH + x;
            unsafe {
                let char_pointer = buffer.offset(offset as isize);
                let prev_char_pointer = buffer.offset((offset - BUFFER_WIDTH) as isize);
                *prev_char_pointer = *char_pointer;
            }
        }
    }
    for x in 0..BUFFER_WIDTH {
        set_char(x, BUFFER_HEIGHT - 1, b' ', VgaColor::White, VgaColor::Black);
    }
}