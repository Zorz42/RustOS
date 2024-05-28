use core::arch::asm;
use std::{memcpy_non_aligned, memset, String, Vec};
use crate::keyboard::{get_key_event, Key, key_to_char};
use crate::{print, println};
use crate::disk::filesystem::{get_fs, Path};
use crate::memory::{map_page, map_page_auto, PAGE_SIZE, VirtAddr};
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

fn command_erase() {
    println!("Erasing the disk");
    get_fs().erase();
}

#[repr(C)]
struct ElfHeader {
    ident: [u8; 16],
    file_type: u16,
    machine: u16,
    version: u32,
    entry: u64,
    program_header_offset: u64,
    section_header_offset: u64,
    flags: u32,
    header_size: u16,
    program_header_entry_size: u16,
    program_header_num_entries: u16,
    section_header_entry_size: u16,
    section_header_num_entries: u16,
    section_names_offset: u16,
}

#[repr(C)]
struct ProgramHeader {
    header_type: u32,
    flags: u32,
    offset: u64,
    virt_addr: u64,
    phys_addr: u64,
    size_in_file: u64,
    size_in_memory: u64,
    align: u64,
}


fn verify_elf_header(header: &ElfHeader) -> bool {
        header.ident[0] == 0x7F &&
        header.ident[1] as char == 'E' &&
        header.ident[2] as char == 'L' &&
        header.ident[3] as char == 'F'
}

struct MemoryRange {
    start: u64,
    length: u64,
    file_offset: Option<u64>,
}

const USER_STACK: u64 = 0x30000000000;
const USER_STACK_SIZE: u64 = 100 * 1024; // 100kB

unsafe fn switch_to_user_mode(entry_point: u64, user_stack: u64) {
    asm!(
    "
        cli                  // Clear interrupts
        mov ax, 0x23         // User data segment selector (DS, ES, FS, GS)
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax
        push 0x23            // User data segment selector (SS)
        push {1}             // User stack pointer
        pushf                // EFLAGS
        push 0x1B            // User code segment selector (CS)
        push {0}             // Entry point address
        iretq                // Interrupt return, switches to user mode
        ",
    in(reg) entry_point,
    in(reg) user_stack,
    options(noreturn)
    );
}

fn run_program(name: String) {
    // run program
    let mut file_path = String::from("programs/");
    for c in &name {
        file_path.push(*c);
    }
    let file = if let Some(file) = get_fs().get_file(&file_path) {
        file
    } else {
        println!("Unknown command \"{name}\"");
        return;
    };
    
    let testing_program = file.read();
    if testing_program.size() < core::mem::size_of::<ElfHeader>() {
        println!("\"{name}\" is too small to contain an ELF header.");
        return;
    }
    
    let mut elf_header = unsafe { &*(testing_program.as_ptr() as *const ElfHeader) };
    
    if !verify_elf_header(elf_header) {
        println!("\"{name}\" has invalid ELF header");
        return;
    }

    let mut ranges = Vec::new();
    
    for i in 0..elf_header.program_header_num_entries {
        let addr = unsafe { testing_program.as_ptr().add(elf_header.program_header_offset as usize).add(i as usize * elf_header.program_header_entry_size as usize) as *const ProgramHeader };
        let header = unsafe { &*(addr) };
        
        let mut range = MemoryRange {
            start: header.virt_addr,
            length: header.size_in_memory,
            file_offset: None,
        };
        
        if header.size_in_file != 0 {
            range.file_offset = Some(header.offset);
        }
        
        assert!(header.size_in_file == 0 || header.size_in_file == header.size_in_memory);
        
        if header.header_type == 1 {
            if range.length != 0 {
                ranges.push(range);
            }
        } else if (header.header_type >= 0x60000000 && header.header_type <= 0x7FFFFFFF) || header.header_type == 0 || header.header_type == 2 {
            // just ignore it
        } else {
            panic!("Unknown header type {}", header.header_type)
        }
    }
    
    ranges.sort(&|a, b| a.start < b.start);
    
    // check for ranges not overlapping
    let mut high_addr = 0;
    for range in &ranges {
        assert!(high_addr <= range.start);
        high_addr = range.start + range.length;
    }
    
    // map pages
    let mut curr_page = 0;
    for range in &ranges {
        curr_page = u64::max(curr_page, range.start / PAGE_SIZE * PAGE_SIZE);
        
        while curr_page < range.start + range.length {
            map_page_auto(curr_page as VirtAddr, true, true);
            curr_page += PAGE_SIZE;
        }
    }
    
    // copy data to memory
    for range in &ranges {
        if let Some(file_offset) = range.file_offset {
            unsafe {
                memcpy_non_aligned(testing_program.as_ptr().add(file_offset as usize), range.start as *mut u8, range.length as usize);
            }
        } else {
            unsafe {
                memset(range.start as *mut u8, 0, range.length as usize);
            }
        }
    }
    
    // map stack 
    let num_pages_stack = (USER_STACK_SIZE + PAGE_SIZE - 1) / PAGE_SIZE;
    for i in 0..num_pages_stack {
        map_page_auto((USER_STACK - (num_pages_stack - i) * PAGE_SIZE) as VirtAddr, true, true);
    }
    
    unsafe {
        /*let entry = elf_header.entry;
        asm!("call {}", in(reg) entry);*/
        
        switch_to_user_mode(elf_header.entry, USER_STACK);
        
        println!("Going to infinite loop");
        loop {
            asm!("hlt");
        }

        /*let rax: u64;
        asm!("mov {}, rax", out(reg) rax);
        println!("Program exited with code {rax}");*/
    }
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
        "erase" => command_erase(),
        _ => {
            run_program(command_name);
        },
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