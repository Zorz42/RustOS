mod bitset;

pub use bitset::{BitSetRaw, bitset_size_bytes};

extern "C" {
    pub static _end: u8;
}

pub fn get_kernel_top_address() -> u64 {
    unsafe { &_end as *const u8 as u64 }
}

#[repr(C)]
struct FdtHeader {
    magic: u32,
    totalsize: u32,
    off_dt_struct: u32,
    off_dt_strings: u32,
    off_mem_rsvmap: u32,
    version: u32,
    last_comp_version: u32,
    boot_cpuid_phys: u32,
    size_dt_strings: u32,
    size_dt_struct: u32,
}

const FDT_MAGIC: u32 = 0xd00dfeed;
const FDT_BEGIN_NODE: u32 = 0x1;
const FDT_END_NODE: u32 = 0x2;
const FDT_PROP: u32 = 0x3;
const FDT_NOP: u32 = 0x4;
const FDT_END: u32 = 0x9;

#[repr(C)]
struct FdtProp {
    len: u32,
    nameoff: u32,
}

unsafe fn from_be_u32(ptr: *const u8) -> u32 {
    u32::from_be(*(ptr as *const u32))
}

pub fn parse_dtb(dtb: *const u8) -> usize {
    unsafe {
        let header = &*(dtb as *const FdtHeader);
        if u32::from_be(header.magic) != FDT_MAGIC {
            panic!("Invalid DTB magic number");
        }

        let struct_start = dtb.add(u32::from_be(header.off_dt_struct) as usize);
        let strings_start = dtb.add(u32::from_be(header.off_dt_strings) as usize);
        let struct_end = struct_start.add(u32::from_be(header.size_dt_struct) as usize);

        let mut ptr = struct_start;
        let mut total_memory = 0;

        while ptr < struct_end {
            let token = from_be_u32(ptr);
            ptr = ptr.add(4);

            match token {
                FDT_BEGIN_NODE => {
                    while *ptr != 0 {
                        ptr = ptr.add(1);
                    }
                    ptr = ptr.add(1); // Skip the null terminator
                    ptr = (ptr.add(3) as usize & !3) as *mut u8; // Align to 4 bytes
                }
                FDT_END_NODE => {}
                FDT_PROP => {
                    let prop = &*(ptr as *const FdtProp);
                    ptr = ptr.add(core::mem::size_of::<FdtProp>());
                    let name = strings_start.add(u32::from_be(prop.nameoff) as usize);
                    let name = core::ffi::CStr::from_ptr(name as *const i8).to_str().unwrap();
                    if name == "reg" {
                        let len = u32::from_be(prop.len) as usize;
                        let reg_data = core::slice::from_raw_parts(ptr, len);
                        if len >= 16 {
                            let size = usize::from_be_bytes([
                                reg_data[8], reg_data[9], reg_data[10], reg_data[11],
                                reg_data[12], reg_data[13], reg_data[14], reg_data[15],
                            ]);
                            total_memory += size;
                        }
                    }
                    ptr = ptr.add(u32::from_be(prop.len) as usize);
                    ptr = (ptr.add(3) as usize & !3) as *mut u8; // Align to 4 bytes
                }
                FDT_NOP => {}
                FDT_END => break,
                _ => panic!("Unknown FDT token: {}", token),
            }
        }

        total_memory
    }
}
