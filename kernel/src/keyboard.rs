use crate::interrupts::{set_idt_entry, ExceptionStackFrame};
use crate::ports::{byte_in, byte_out};
use crate::println;

const QUEUE_SIZE: usize = 1024;
static mut QUEUE: [u8; QUEUE_SIZE] = [0; QUEUE_SIZE];
static mut QUEUE_TOP: usize = 0;
static mut QUEUE_BOTTOM: usize = 0;

pub fn init_keyboard() {
    set_idt_entry(33, keyboard_handler);
}

// (key code, is up)
pub fn get_key_event() -> Option<(Key, bool)> {
    unsafe {
        if QUEUE_TOP == QUEUE_BOTTOM {
            None
        } else {
            let res = QUEUE[QUEUE_BOTTOM];
            QUEUE_BOTTOM = (QUEUE_BOTTOM + 1) % QUEUE_SIZE;
            let code = res & 0x7F;
            
            let key = scancode_to_key(code);
            
            Some((key, (res & 0x80) != 0))
        }
    }
}

pub fn key_to_char(code: Key) -> Option<char> {
    match code {
        Key::A => Some('a'),
        Key::B => Some('b'),
        Key::C => Some('c'),
        Key::D => Some('d'),
        Key::E => Some('e'),
        Key::F => Some('f'),
        Key::G => Some('g'),
        Key::H => Some('h'),
        Key::I => Some('i'),
        Key::J => Some('j'),
        Key::K => Some('k'),
        Key::L => Some('l'),
        Key::M => Some('m'),
        Key::N => Some('n'),
        Key::O => Some('o'),
        Key::P => Some('p'),
        Key::Q => Some('q'),
        Key::R => Some('r'),
        Key::S => Some('s'),
        Key::T => Some('t'),
        Key::U => Some('u'),
        Key::V => Some('v'),
        Key::W => Some('w'),
        Key::X => Some('x'),
        Key::Y => Some('y'),
        Key::Z => Some('z'),
        Key::Num0 => Some('0'),
        Key::Num1 => Some('1'),
        Key::Num2 => Some('2'),
        Key::Num3 => Some('3'),
        Key::Num4 => Some('4'),
        Key::Num5 => Some('5'),
        Key::Num6 => Some('6'),
        Key::Num7 => Some('7'),
        Key::Num8 => Some('8'),
        Key::Num9 => Some('9'),

        Key::Minus => Some('-'),
        Key::Plus => Some('+'),
        Key::LeftBracket => Some(']'),
        Key::RightBracket => Some('['),
        Key::Semicolon => Some(';'),
        Key::Quote => Some('\''),
        Key::Backquote => Some('`'),
        Key::Period => Some('.'),
        Key::Comma => Some(','),
        Key::Slash => Some('/'),
        Key::Space => Some(' '),
        Key::Backslash => Some('\\'),

        _ => None
    }
}

extern "x86-interrupt" fn keyboard_handler(_stack_frame: &ExceptionStackFrame) {
    byte_out(0x20, 0x20);
    let code = byte_in(0x60);
    unsafe {
        QUEUE[QUEUE_TOP] = code;
        QUEUE_TOP = (QUEUE_TOP + 1) % QUEUE_SIZE;
    }
}

fn scancode_to_key(scancode: u8) -> Key {
    match scancode {
        0x0B => Key::Num0,
        0x02 => Key::Num1,
        0x03 => Key::Num2,
        0x04 => Key::Num3,
        0x05 => Key::Num4,
        0x06 => Key::Num5,
        0x07 => Key::Num6,
        0x08 => Key::Num7,
        0x09 => Key::Num8,
        0x0A => Key::Num9,
        
        0x1E => Key::A,
        0x30 => Key::B,
        0x2E => Key::C,
        0x20 => Key::D,
        0x12 => Key::E,
        0x21 => Key::F,
        0x22 => Key::G,
        0x23 => Key::H,
        0x17 => Key::I,
        0x24 => Key::J,
        0x25 => Key::K,
        0x26 => Key::L,
        0x32 => Key::M,
        0x31 => Key::N,
        0x18 => Key::O,
        0x19 => Key::P,
        0x10 => Key::Q,
        0x13 => Key::R,
        0x1F => Key::S,
        0x14 => Key::T,
        0x16 => Key::U,
        0x2F => Key::V,
        0x11 => Key::W,
        0x2D => Key::X,
        0x15 => Key::Y,
        0x2C => Key::Z,

        0x0C => Key::Minus,
        0x0D => Key::Plus,
        0x1B => Key::LeftBracket,
        0x1A => Key::RightBracket,
        0x0F => Key::Tab,
        0x27 => Key::Semicolon,
        0x28 => Key::Quote,
        0x29 => Key::Backquote,
        0x34 => Key::Period,
        0x33 => Key::Comma,
        0x35 => Key::Slash,
        0x39 => Key::Space,
        0x2B => Key::Backslash,

        0x0E => Key::Backspace,
        0x01 => Key::Escape,
        0x1C => Key::Enter,
        0x1D => Key::LCtrl,
        0x2A => Key::LShift,
        0x36 => Key::RShift,
        0x38 => Key::LAlt,
        
        _ => Key::Unknown,
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Key {
    Unknown = 0x00,

    // numbers
    Num0 = 0x0B,
    Num1 = 0x02,
    Num2 = 0x03,
    Num3 = 0x04,
    Num4 = 0x05,
    Num5 = 0x06,
    Num6 = 0x07,
    Num7 = 0x08,
    Num8 = 0x09,
    Num9 = 0x0A,
    
    // letters
    A = 0x1E,
    B = 0x30,
    C = 0x2E,
    D = 0x20,
    E = 0x12,
    F = 0x21,
    G = 0x22,
    H = 0x23,
    I = 0x17,
    J = 0x24,
    K = 0x25,
    L = 0x26,
    M = 0x32,
    N = 0x31,
    O = 0x18,
    P = 0x19,
    Q = 0x10,
    R = 0x13,
    S = 0x1F,
    T = 0x14,
    U = 0x16,
    V = 0x2F,
    W = 0x11,
    X = 0x2D,
    Y = 0x15,
    Z = 0x2C,

    // non letter or number characters
    Minus = 0x0C,
    Plus = 0x0D,
    LeftBracket = 0x1B,
    RightBracket = 0x1A,
    Tab = 0x0F,
    Semicolon = 0x27,
    Quote = 0x28,
    Backquote = 0x29,
    Period = 0x34,
    Comma = 0x33,
    Slash = 0x35,
    Space = 0x39,
    Backslash = 0x2B,

    // special keys
    Backspace = 0x0E,
    Escape = 0x01,
    Enter = 0x1C,
    LCtrl = 0x1D,
    LShift = 0x2A,
    RShift = 0x36,
    LAlt = 0x38,
}
