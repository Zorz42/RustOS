const FAULT_STACK_SIZE: usize = 4096 * 5;

pub const DOUBLE_FAULT_IST: u16 = 1;
pub const NMI_IST: u16 = 2;
pub const MACHINE_CHECK_IST: u16 = 3;

const GDT_SIZE: usize = 8;

#[repr(C, packed)]
struct SegmentDescriptor {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access_byte: u8,
    lim_h_flags: u8,
    base_high: u8,
}

#[repr(C, packed)]
struct GDTPointer {
    limit: u16,
    base: u64,
}

#[used]
static mut GDT_POINTER: GDTPointer = GDTPointer { limit: 0, base: 0 };

#[repr(C, packed)]
struct TaskStateSegment {
    _padding_1: u32,
    privilege_stack_table: [u64; 3],
    _padding_2: u64,
    interrupt_stack_table: [u64; 7],
    _padding_3: u64,
    _padding_4: u16,
    io_map_base_address: u16,
}

#[repr(align(16))]
#[derive(Copy, Clone)]
struct Ist {
    #[allow(unused)] //used
    stack: [u8; FAULT_STACK_SIZE],
}

#[used]
static mut TSS: TaskStateSegment = TaskStateSegment {
    _padding_1: 0,
    privilege_stack_table: [0; 3],
    _padding_2: 0,
    interrupt_stack_table: [0; 7],
    _padding_3: 0,
    _padding_4: 0,
    io_map_base_address: core::mem::size_of::<TaskStateSegment>() as u16,
};

fn create_tss() -> TaskStateSegment {
    let mut res = TaskStateSegment {
        _padding_1: 0,
        privilege_stack_table: [0; 3],
        _padding_2: 0,
        interrupt_stack_table: [0; 7],
        _padding_3: 0,
        _padding_4: 0,
        io_map_base_address: core::mem::size_of::<TaskStateSegment>() as u16,
    };

    static mut STACKS: [Ist; 3] = [Ist { stack: [0; FAULT_STACK_SIZE] }; 3];

    for i in 0..3 {
        res.interrupt_stack_table[i] = {
            unsafe {
                core::ptr::addr_of!(STACKS[i]) as u64 + FAULT_STACK_SIZE as u64
            }
        };
    }

    res
}

#[repr(C, align(8))]
struct GlobalDescriptorTable {
    table: [SegmentDescriptor; GDT_SIZE],
    len: usize,
}


const ARRAY_REPEAT_VALUE: SegmentDescriptor = create_segment_descriptor(0, 0, 0, 0);

#[used]
static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable {
    table: [ARRAY_REPEAT_VALUE; GDT_SIZE],
    len: 0,
};

impl GlobalDescriptorTable {
    fn create_pointer(&self) -> GDTPointer {
        GDTPointer {
            limit: (GDT_SIZE * 8 - 1) as u16,
            base: self.table.as_ptr() as u64,
        }
    }

    fn append(&mut self, obj: SegmentDescriptor) {
        assert!(self.len < GDT_SIZE);
        self.table[self.len] = obj;
        self.len += 1;
    }
}

const fn create_segment_descriptor(base: u64, limit: u32, access_byte: u8, flags: u8) -> SegmentDescriptor {
    SegmentDescriptor {
        limit_low: (limit & 0xFFFF) as u16,
        base_low: (base & 0xFFFF) as u16,
        base_mid: ((base & 0xFF0000) >> 16) as u8,
        access_byte,
        lim_h_flags: ((limit & 0xF0000) >> 16) as u8 | ((flags & 0xF) << 4),
        base_high: ((base & 0xFF000000) >> 24) as u8,
    }
}

const fn create_128_segment_descriptor(base: u64, limit: u32, access_byte: u8, flags: u8) -> (SegmentDescriptor, SegmentDescriptor) {
    let low = create_segment_descriptor(base, limit, access_byte, flags);
    let high = create_segment_descriptor((base >> 48) & 0xFFFF, ((base >> 32) & 0xFFFF) as u32, 0, 0);

    (low, high)
}

fn load_gdt(gdt_pointer: *const GDTPointer) {
    unsafe {
        core::arch::asm!("lgdt [{}]", in(reg) gdt_pointer, options(readonly, nostack, preserves_flags));
    }
}

pub fn init_gdt() {
    unsafe {
        TSS = create_tss();
        
        GDT.append(create_segment_descriptor(0, 0, 0, 0));
        GDT.append(create_segment_descriptor(0, 0xFFFFF, 0x9B, 0xA));
        GDT.append(create_segment_descriptor(0, 0xFFFFF, 0x93, 0xC));
        GDT.append(create_segment_descriptor(0, 0xFFFFF, 0xFB, 0xA));
        GDT.append(create_segment_descriptor(0, 0xFFFFF, 0xF3, 0xC));
        
        let desc = create_128_segment_descriptor(
            core::ptr::addr_of!(TSS) as u64,
            (core::mem::size_of::<TaskStateSegment>() - 1) as u32,
            0x89,
            0x0,
        );
        GDT.append(desc.0);
        GDT.append(desc.1);
    };

    unsafe {
        GDT_POINTER = GDT.create_pointer();
        load_gdt(core::ptr::addr_of!(GDT_POINTER));
        set_cs();
        core::arch::asm!("mov ax, 0x28", "ltr ax", out("ax") _, options(nostack, preserves_flags, raw));
    }
}

fn set_cs() {
    unsafe {
        core::arch::asm!(
        "mov ax, 0x10",
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",
        "mov ss, ax",
        out("ax") _,
        options(preserves_flags),
        );
    }
}