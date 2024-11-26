use core::arch::asm;
use kernel_std::{print, println, String};
use crate::input::{check_for_virtio_input_event, keycode_to_char, EventType};
use crate::print::check_screen_refresh_for_print;
use crate::timer::get_ticks;

fn render_line(line: &String, show_cursor: bool) {
    print!("\r> {}", line);
    if show_cursor {
        print!("_");
    }
}

fn on_command(command: &String) {
    if command.size() == 0 {
        return;
    }

    println!("Command: {}", command);
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
                    on_command(&command);
                    if command == String::from("exit") {
                        break 'console_loop;
                    }
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