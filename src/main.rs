#![no_std]
#![no_main]
#![feature(core_intrinsics)]

use core::intrinsics::volatile_set_memory;
use core::panic::PanicInfo;
use core::ptr::write;

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
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

fn write_char(x: usize, y: usize, c: u8) {
    let offset = y * BUFFER_WIDTH + x;
    unsafe {
        *VGA_BUFFER.offset((offset * 2) as isize) = c;
    }
}

fn write_char_color(x: usize, y: usize, text_color: VgaColor, background_color: VgaColor) {
    let offset = y * BUFFER_WIDTH + x;
    unsafe {
        let vga_color = VGA_BUFFER.offset((offset * 2 + 1) as isize);
        *vga_color = (background_color as u8) << 4 | (text_color as u8);
    }
}

struct Writer {
    x: usize,
    y: usize,
}

impl Writer {
    pub const fn new() -> Writer {
        Writer { 
            x: 0, 
            y: 0,
        }
    }
    
    pub fn clear_screen(&mut self) {
        for y in 0..BUFFER_HEIGHT {
            for x in 0..BUFFER_WIDTH {
                write_char(x, y, b' ');
                write_char_color(x, y, VgaColor::White, VgaColor::Black);
            }
        }
        self.x = 0;
        self.y = 0;
    }
    
    pub fn write_char(&mut self, c: u8) {
        write_char(self.x, self.y, c);
        self.x += 1;
        if self.x >= BUFFER_WIDTH {
            self.x = 0;
            self.y += 1;
        }
    }

    pub fn write_char_colored(&mut self, c: u8, text_color: VgaColor, background_color: VgaColor) {
        write_char_color(self.x, self.y, text_color, background_color);
        self.write_char(c);
    }
}

static mut WRITER: Writer = Writer::new();

fn init_writer() {
    unsafe {
        WRITER.clear_screen();
    }
}

fn print_char(c: u8) {
    unsafe {
        WRITER.write_char(c);
    }
}

fn print(c: &str) {
    for b in c.bytes() {
        print_char(b);
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_writer();
    
    print("Hello, World!");

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}