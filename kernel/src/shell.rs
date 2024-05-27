use core::arch::asm;
use std::{memcpy_non_aligned, String, Vec};
use crate::keyboard::{get_key_event, Key, key_to_char};
use crate::{print, println};
use crate::disk::filesystem::{get_fs, Path};
use crate::memory::{map_page_auto, PAGE_SIZE, VirtAddr};
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

const EI_MAG0: usize = 0; // File identification
const EI_MAG1: usize = 1; // File identification
const EI_MAG2: usize = 2; // File identification
const EI_MAG3: usize = 3; // File identification
const EI_CLASS: usize = 4; // File class
const EI_DATA: usize = 5; // Data encoding
const EI_VERSION: usize = 6; // File version
const EI_OSABI: usize = 7; // Operating system/ABI identification
const EI_ABIVERSION: usize = 8; // ABI version
const EI_PAD: usize = 9; // Start of padding bytes
const EI_NIDENT: usize = 16; // Size of e_ident[]

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
    program_section_num_entries: u16,
    section_names_offset: u16,
}

#[repr(C)]
#[derive(Debug)]
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

#[repr(C)]
#[derive(Debug)]
struct SectionHeader {
    header_type: u32,
    flags: u64,
    virt_addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    align: u64,
    entry_size: u64,
}

fn verify_elf_header(header: &ElfHeader) -> bool {
        header.ident[EI_MAG0] == 0x7F &&
        header.ident[EI_MAG1] as char == 'E' &&
        header.ident[EI_MAG2] as char == 'L' &&
        header.ident[EI_MAG3] as char == 'F'
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
    
    println!("{}", elf_header.file_type);
    
    if !verify_elf_header(elf_header) {
        println!("\"{name}\" has invalid ELF header");
        return;
    }
    
    
    println!("Num {} {}", elf_header.program_header_num_entries, elf_header.program_section_num_entries);

    for i in 0..elf_header.program_header_num_entries {
        let addr = unsafe { testing_program.as_ptr().add(elf_header.program_header_offset as usize).add(i as usize * elf_header.program_header_entry_size as usize) as *const ProgramHeader };
        let header = unsafe { &*(addr) };
        println!("{:?}", header);
    }

    for i in 0..elf_header.program_section_num_entries {
        let addr = unsafe { testing_program.as_ptr().add(elf_header.section_header_offset as usize).add(i as usize * elf_header.section_header_entry_size as usize) as *const SectionHeader };
        let header = unsafe { &*(addr) };
        println!("{:?}", header);
    }
    
    /*let mut entry = 0x1000;
    for i in 0..8 {
        entry += (testing_program[24 + i] as u64) << (i * 8);
    }
    let program_offset = 1u64 << (12 + 3 * 9 + 2);

    let num_pages = (testing_program.size() as u64 + PAGE_SIZE - 1) / PAGE_SIZE;
    println!("allocating {num_pages} pages");
    for i in 0..num_pages {
        map_page_auto((program_offset + PAGE_SIZE * i) as VirtAddr, true, true);
    }

    unsafe {
        memcpy_non_aligned(testing_program.as_ptr(), program_offset as *mut u8, testing_program.size());
        
        println!("Calling address 0x{entry:x}");
        asm!("call {}", in(reg) entry);

        let rax: u64;
        asm!("mov {}, rax", out(reg) rax);
        println!("Program exited with code {rax}");
    }*/
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