use core::arch::asm;

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

#[repr(align(16))]
struct Stack([u8; STACK_SIZE]);

#[used]
#[no_mangle]
static mut KERNEL_STACK: Stack = Stack([0; STACK_SIZE]);

#[no_mangle]
extern "C" fn start() {
    let val = 10;

    loop {}
}