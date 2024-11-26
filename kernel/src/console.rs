use core::arch::asm;
use kernel_std::{print, println, String, Vec};
use crate::disk::filesystem::{read_file, write_to_file};
use crate::input::{check_for_virtio_input_event, keycode_to_char, EventType};
use crate::print::check_screen_refresh_for_print;
use crate::timer::get_ticks;

fn render_line(line: &String, show_cursor: bool) {
    print!("\r> {}", line);
    if show_cursor {
        print!("_");
    }
}

fn cp_command(parts: Vec<String>) {
    if parts.size() != 2 {
        println!("Usage: cp <source> <destination>");
        return;
    }

    let source = &parts[0];
    let destination = &parts[1];

    let data = if let Some(data) = read_file(source) {
        data
    } else {
        println!("Source file not found: \"{}\"", source);
        return;
    };
    write_to_file(destination, &data);
}

fn on_command(mut command: String) {
    command.push(' ');
    let mut command_parts = Vec::new();
    let mut current_part = String::new();
    for c in &command {
        if *c == ' ' {
            if current_part.size() > 0 {
                command_parts.push(current_part);
                current_part = String::new();
            }
        } else {
            current_part.push(*c);
        }
    }

    if command_parts.size() == 0 {
        return;
    }

    let command = command_parts[0].clone();
    command_parts.reverse();
    command_parts.pop();
    command_parts.reverse();

    if command == String::from("help") {
        println!("Commands:");
        println!("  help - show this help");
        println!("  exit - exit console");
    } else if command == String::from("cp") {
        cp_command(command_parts);
    } else {
        println!("Unknown command: {}", command);
    }
}

const CURSOR_BLINK_INTERVAL: u64 = 500;

pub fn run_console() {
    println!("Running console");

    let mut command = String::new();
    let mut cursor_shown = true;
    render_line(&command, cursor_shown);

    let mut prev_cursor_cycle = get_ticks();
    'console_loop: loop {
        check_screen_refresh_for_print();

        while let Some(event) = check_for_virtio_input_event() {
            if event.event_type == EventType::Key && event.value == 1 {
                if let Some(c) = keycode_to_char(event.code) {
                    command.push(c);
                    render_line(&command, cursor_shown);
                }

                // Handle backspace
                if event.code == 14 && command.size() > 0 {
                    command.pop();
                    render_line(&command, cursor_shown);
                }

                // Handle enter
                if event.code == 28 {
                    render_line(&command, false);
                    println!();
                    if command == String::from("exit") {
                        println!("Exiting console");
                        break 'console_loop;
                    }
                    on_command(command);
                    command = String::new();
                    render_line(&command, cursor_shown);
                }
            }
        }

        if get_ticks() - prev_cursor_cycle > CURSOR_BLINK_INTERVAL {
            cursor_shown = !cursor_shown;
            render_line(&command, cursor_shown);
            prev_cursor_cycle = get_ticks();
        }

        unsafe {
            asm!("wfi");
        }
    }
}