use core::arch::asm;
use std::{String, Vec};
use crate::keyboard::{get_key_event, Key, key_to_char};
use crate::{print, println};
use crate::disk::filesystem::{get_fs, Path};
use crate::print::move_cursor_back;

struct Context {
    curr_dir: Path,
}

fn command_ls(args: Vec<String>, context: &Context) {
    if args.size() != 0 {
        println!("Expected 0 arguments");
        return;
    }
    
    let curr_dir = get_fs().get_directory(&context.curr_dir.to_string()).unwrap();
    for dir in curr_dir.get_directories() {
        println!("{}/", dir.get().get_name());
    }
    
    for file in curr_dir.get_files() {
        println!("{}", file.get_name());
    }
}

fn command_mkdir(args: Vec<String>, context: &Context) {
    let curr_dir = get_fs().get_directory(&context.curr_dir.to_string()).unwrap();
    for dir in args {
        curr_dir.create_directory(dir);
    }
}

fn command_cd(args: Vec<String>, context: &mut Context) {
    if args.size() != 1 {
        println!("Expected 1 argument");
        return;
    }
    
    let dest = if args[0][0] == '/' {
        args[0].clone()
    } else {
        let mut res = context.curr_dir.to_string();
        for c in &args[0] {
            res.push(*c);
        }
        res
    };
    
    if get_fs().get_directory(&dest).is_none() {
        println!("{dest} is not a directory");
        return;
    }
    
    context.curr_dir = Path::from(&dest);
}

fn command_cp(args: Vec<String>, context: &mut Context) {
    if args.size() != 2 {
        println!("Expected 2 arguments");
        return;
    }

    let src = if args[0][0] == '/' {
        args[0].clone()
    } else {
        let mut res = context.curr_dir.to_string();
        for c in &args[0] {
            res.push(*c);
        }
        res
    };

    let dest = if args[1][0] == '/' {
        args[1].clone()
    } else {
        let mut res = context.curr_dir.to_string();
        for c in &args[1] {
            res.push(*c);
        }
        res
    };

    if get_fs().get_file(&src).is_none() {
        println!("{src} is not a file");
        return;
    }
    
    let data = get_fs().get_file(&src).unwrap().read();
    let dst_file = if let Some(file) = get_fs().get_file(&dest) {
        file
    } else {
        get_fs().create_file(&dest)
    };
    
    dst_file.write(&data);

    context.curr_dir = Path::from(&dest);
}

fn command_callback(command: String, context: &mut Context) {
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
        "ls" => command_ls(parts, context),
        "mkdir" => command_mkdir(parts, context),
        "cd" => command_cd(parts, context),
        "cp" => command_cp(parts, context),
        _ => println!("Unknown command \"{command_name}\""),
    }
}

pub fn shell_main() {
    let mut context = Context {
        curr_dir: Path::from(&String::new())
    };
    
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

                    
                    command_callback(command.clone(), &mut context);
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