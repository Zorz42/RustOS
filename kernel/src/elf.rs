#[derive(Debug)]
#[repr(C)]
pub struct ElfHeader {
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
pub struct ElfProgramHeader {
    pub p_type: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub align: u64,
}

pub fn verify_elf_header(header: &ElfHeader) -> bool {
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