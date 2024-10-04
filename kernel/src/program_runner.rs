use std::{println, String, Vec};
use crate::disk::filesystem::get_fs;
use crate::memory::{map_page_auto, VirtAddr, KERNEL_VIRTUAL_TOP, PAGE_SIZE};

#[derive(Debug)]
#[repr(C)]
struct ElfHeader {
    pub magic: [u8; 4],
    pub bits: u8,
    pub endianness: u8,
    pub version: u8,
    pub abi: u8,
    pub abi_version: u8,
    pub padding: [u8; 7],
    pub elf_type: u16,
    pub machine: u16,
    pub version2: u32,
    pub entry: u64,
    pub ph_offset: u64,
    pub sh_offset: u64,
    pub flags: u32,
    pub header_size: u16,
    pub ph_entry_size: u16,
    pub ph_entry_count: u16,
    pub sh_entry_size: u16,
    pub sh_entry_count: u16,
    pub sh_str_index: u16,
}

#[derive(Debug)]
#[repr(C)]
struct ElfProgramHeader {
    pub p_type: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub align: u64,
}

fn verify_elf_header(header: &ElfHeader) -> bool {
    if header.magic != [0x7f, 0x45, 0x4c, 0x46] {
        return false;
    }

    if header.bits != 2 {
        return false;
    }

    if header.endianness != 1 {
        return false;
    }

    if header.version != 1 {
        return false;
    }

    if header.abi != 0 {
        return false;
    }

    if header.abi_version != 0 {
        return false;
    }

    if header.elf_type != 2 {
        return false;
    }

    if header.machine != 0xf3 {
        return false;
    }

    if header.version2 != 1 {
        return false;
    }

    true
}

pub fn run_program(path: &String) {
    println!("Running program: {}", path);

    let program = get_fs().get_file(path).unwrap().read();
    println!("Program size {}", program.size());
    let elf_header = unsafe { (program.as_ptr() as *const ElfHeader).read() };

    if !verify_elf_header(&elf_header) {
        println!("Invalid ELF header");
        return;
    }

    // get program headers
    let mut program_headers = Vec::new();
    for i in 0..elf_header.ph_entry_count {
        let program_header = unsafe { (program.as_ptr().add(elf_header.ph_offset as usize) as *const ElfProgramHeader).add(i as usize).read() };
        program_headers.push(program_header);
    }

    //println!("Elf header: {:?}", elf_header);
    /*println!("Program headers: ");
    for header in &program_headers {
        println!("{:?}", header);
    }*/

    // map program headers to memory
    for header in &program_headers {
        if header.p_type == 1 && header.memory_size != 0 {
            assert!(header.vaddr >= KERNEL_VIRTUAL_TOP);

            let low_page = header.vaddr / PAGE_SIZE;
            let high_page = (header.vaddr + header.memory_size + PAGE_SIZE - 1) / PAGE_SIZE;
            for page in low_page..high_page {
                map_page_auto((page * PAGE_SIZE) as VirtAddr, true, header.flags & 4 != 0, header.flags & 2 != 0, false, header.flags & 1 != 0);
            }

            let ptr_low = header.vaddr as *mut u8;
            let ptr_mid = (header.vaddr + header.file_size) as *mut u8;
            let ptr_high = (header.vaddr + header.memory_size) as *mut u8;
            unsafe {
                core::ptr::copy(program.as_ptr().add(header.offset as usize), ptr_low, header.file_size as usize);
                if ptr_mid != ptr_high {
                    core::ptr::write_bytes(ptr_mid, 0, (header.memory_size - header.file_size) as usize);
                }
            }
        }
    }

    // run the program
    let code: fn() -> i32 = unsafe { core::mem::transmute(elf_header.entry) };
    let result = code();
    println!("Program returned: {}", result);
}
