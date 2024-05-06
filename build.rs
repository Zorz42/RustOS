use std::path::PathBuf;

#[allow(clippy::unwrap_used)]
fn main() {
    // set by cargo, build scripts should use this directory for output files
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    // set by cargo's artifact dependency feature, see
    // https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#artifact-dependencies
    let kernel = PathBuf::from(std::env::var_os("CARGO_BIN_FILE_KERNEL_kernel").unwrap());

    // create an UEFI disk image (optional)
    let uefi_path = out_dir.join("uefi.img");
    bootloader::UefiBoot::new(&kernel).create_disk_image(&uefi_path).unwrap();

    // create a BIOS disk image
    let bios_path = out_dir.join("bios.img");
    bootloader::BiosBoot::new(&kernel).create_disk_image(&bios_path).unwrap();

    // pass the disk image paths as env variables to the `main.rs`
    println!("cargo:rustc-env=UEFI_PATH={}", uefi_path.display());
    println!("cargo:rustc-env=BIOS_PATH={}", bios_path.display());

    const NUM_SECTORS: usize = 1000;
    let mut testdisk_data = [0u8; NUM_SECTORS * 512];
    const MAGIC_CODE: u32 = 0x61732581;
    for i in 0..4 {
        testdisk_data[512 - 4 - i] = ((MAGIC_CODE >> (8 * i)) & 0xFF) as u8;
    }

    std::fs::write("./testdisk.img", testdisk_data).expect("Error writing disk data");
}
