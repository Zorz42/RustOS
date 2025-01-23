use core::arch::asm;

#[repr(u64)]
pub enum SyscallCode {
    PrintStr = 1,
    GetTicks = 2,
    GetPid = 3,
    Exit = 4,
    AllocPage = 5,
    DeallocPage = 6,
    Sleep = 7,
}

pub fn syscall0(code: SyscallCode) {
    unsafe {
        asm!("ecall", in("a7") code as u64);
    }
}

pub fn syscall1(code: SyscallCode, arg1: u64) {
    unsafe {
        asm!("ecall", in("a7") code as u64, in("a3") arg1);
    }
}

pub fn syscall2(code: SyscallCode, arg1: u64, arg2: u64) {
    unsafe {
        asm!("ecall", in("a7") code as u64, in("a3") arg1, in("a4") arg2);
    }
}

pub fn syscall0r(code: SyscallCode) -> u64 {
    let ret: u64;
    unsafe {
        asm!("ecall", in("a7") code as u64, out("a2") ret);
    }
    ret
}