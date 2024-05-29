const STACK_SIZE: usize = 4 * 1024; // 4kB
const NUM_CORES: usize = 4;

fn putchar(c: u8) {
    let addr = 0x10000000 as *mut u8;
    unsafe {
        *addr = c;
    }
}

#[no_mangle]
extern "C" fn rust_entry() {
    /*for c in string.as_bytes() {
        putchar(*c);
    }*/

    putchar('H' as u8);
    putchar('e' as u8);

    //let mut mstatus = get_mstatus();
    //mstatus &= !MODE_MACHINE;
    //mstatus |= MODE_SUPERVISOR;
    //set_mstatus(mstatus);

    loop {}
}