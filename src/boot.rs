use core::arch::asm;
use crate::riscv::{MODE_MACHINE, MODE_SUPERVISOR};

// this has to be the first thing in the binary
#[no_mangle]
#[naked]
extern "C" fn _start() {
    unsafe {
        asm!(r#"
            la sp, KERNEL_STACK
            li a0, 1024*4
            csrr a1, mhartid
            addi a1, a1, 1
            mul a0, a0, a1
            add sp, sp, a0

            call start
        2:
            j 2b

    "#, options(noreturn));
    }
}

const STACK_SIZE: usize = 4 * 1024; // 4kB
const NUM_CORES: usize = 4;

#[repr(align(16))]
struct Stack([u8; NUM_CORES * STACK_SIZE]);

#[used]
#[no_mangle]
static KERNEL_STACK: Stack = Stack([0; NUM_CORES * STACK_SIZE]);

#[no_mangle]
extern "C" fn start() {
    unsafe {
        asm!("call test_fn");
    }

    //let mut mstatus = get_mstatus();
    //mstatus &= !MODE_MACHINE;
    //mstatus |= MODE_SUPERVISOR;
    //set_mstatus(mstatus);
}