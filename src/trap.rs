use crate::riscv::set_stvec;

extern "C" {
    fn kernelvec();
}

#[no_mangle]
extern "C" fn kerneltrap() {
    // TODO: implement
}

pub fn init_trap() {
    set_stvec(kernelvec as u64);
}