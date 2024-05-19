use core::arch::asm;
use std::{String, Vec};
use crate::keyboard::{get_key_event, Key, key_to_char};
use crate::{print, println};
use crate::print::move_cursor_back;

fn command_ls(args: Vec<String>) {
    println!("Listing directories");
}

fn command_callback(command: String) {
    let mut parts = command.split(' ');
    parts.retain(&|x| x.size() != 0);
    parts.reverse();
    let command_name = if let Some(name) = parts.pop() {
        name
    } else {
        return;
    };
    parts.reverse();
    
    match command_name.as_str() {
        "ls" => command_ls(parts),
        _ => println!("Unknown command \"{command_name}\""),
    }
}

pub fn shell_main() {
    print!("\n# _");
    let mut command = String::new();
    'shell_loop: loop {
        while let Some((key, is_up)) = get_key_event() {
            if !is_up {
                if let Some(c) = key_to_char(key) {
                    move_cursor_back();
                    print!("{c}_");
                    command.push(c);
                }

                if key == Key::Enter {
                    move_cursor_back();
                    print!(" \n");
                    if command == String::from("exit") {
                        break 'shell_loop;
                    }

                    
                    command_callback(command.clone());
                    print!("# _");
                    command = String::new();
                }

                if key == Key::Backspace && command.size() != 0 {
                    move_cursor_back();
                    move_cursor_back();
                    print!("  ");
                    move_cursor_back();
                    move_cursor_back();
                    print!("_");
                    command.pop();
                }
            }
        }
        unsafe {
            asm!("hlt");
        }
    }
}