use core::arch::asm;
use core::ptr::{copy, write_bytes, write_volatile};
use std::{println, String, Vec};
use crate::disk::filesystem::get_fs;
use crate::memory::{map_page_auto, VirtAddr, KERNEL_VIRTUAL_TOP, PAGE_SIZE, USER_CONTEXT, USER_STACK};
use crate::riscv::{get_core_id, get_sstatus, interrupts_enable, set_sepc, set_sstatus, SSTATUS_SPP, SSTATUS_UIE};
use crate::trap::switch_to_user_trap;

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

#[derive(Debug)]
#[repr(C)]
pub struct Context {
    pub ra: u64,
    pub sp: u64,
    pub gp: u64,
    pub tp: u64,
    pub t0: u64,
    pub t1: u64,
    pub t2: u64,
    pub s0: u64,
    pub s1: u64,
    pub a0: u64,
    pub a1: u64,
    pub a2: u64,
    pub a3: u64,
    pub a4: u64,
    pub a5: u64,
    pub a6: u64,
    pub a7: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,
    pub t3: u64,
    pub t4: u64,
    pub t5: u64,
    pub t6: u64,
    pub pc: u64,
    pub kernel_hartid: u64,
}

#[derive(Clone, Copy)]
pub struct CpuData {
    pub was_last_interrupt_external: bool,
}

static mut CPU_DATA: [CpuData; 4] = [CpuData { was_last_interrupt_external: false }; 4];

pub fn get_cpu_data() -> &'static mut CpuData {
    unsafe {
        &mut CPU_DATA[get_core_id() as usize]
    }
}

pub fn get_context() -> &'static mut Context {
    unsafe {
        &mut *(USER_CONTEXT as *mut Context)
    }
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
                map_page_auto((page * PAGE_SIZE) as VirtAddr, true, true, true, true);
            }

            assert!(header.memory_size >= header.file_size);
            let ptr_low = header.vaddr as *mut u8;
            let ptr_mid = (header.vaddr + header.file_size) as *mut u8;
            unsafe {
                copy(program.as_ptr().add(header.offset as usize), ptr_low, header.file_size as usize);
                write_bytes(ptr_mid, 0, (header.memory_size - header.file_size) as usize);
            }
        }
    }

    let stack_size = 128 * 1024;
    let stack_pages = stack_size / PAGE_SIZE;
    let stack_top = USER_STACK + stack_size;
    for i in 0..stack_pages {
        map_page_auto((USER_STACK + i * PAGE_SIZE) as VirtAddr, true, true, true, false);
    }

    map_page_auto(USER_CONTEXT as VirtAddr, true, true, true, false);
    unsafe {
        write_bytes(USER_CONTEXT as *mut u8, 0, size_of::<Context>());
    }

    get_context().pc = elf_header.entry;
    get_context().sp = stack_top;

    println!("entry is at {:#x}", elf_header.entry);
}

extern "C" {
    fn jump_to_user() -> !;
}

pub fn jump_to_program() -> ! {
    //interrupts_enable(false);
    // clear bit in sstatus
    set_sstatus(get_sstatus() & !SSTATUS_SPP);

    // set user interrupt enable
    set_sstatus(get_sstatus() | SSTATUS_UIE);

    switch_to_user_trap();
    //interrupts_enable(true);

    unsafe {
        jump_to_user()
    }
}