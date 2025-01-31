use kernel_std::{println, String, Vec, malloc, free};
use kernel_test::{kernel_test, kernel_test_mod};
use crate::disk::filesystem::write_to_file;
use crate::scheduler::{get_num_processes, run_program};
use core::arch::asm;

kernel_test_mod!(crate::tests::B0_scheduler);

#[kernel_test]
fn test_one_process() {
    let test_program = include_bytes!("../../../programs/test_program/target/riscv64gc-unknown-none-elf/release/test_program");
    let test_program_vec = Vec::new_from_slice(test_program);
    write_to_file(&String::from("test_program"), &test_program_vec);


    for i in 0.. 10 {
        assert_eq!(get_num_processes(), 0);

        run_program(&String::from("test_program"));

        assert_eq!(get_num_processes(), 1);

        while get_num_processes() > 0 {
            unsafe {
                asm!("wfi");
            }
        }
    }
}