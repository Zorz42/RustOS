#[allow(clippy::expect_used)]
fn main() {
    //println!("cargo::rerun-if-changed=testdisk.img");

    const NUM_SECTORS: usize = 10000;
    const MAGIC_CODE: u32 = 0x61732581;

    let mut testdisk_data = vec![0u8; NUM_SECTORS * 512];
    for i in 0..4 {
        testdisk_data[512 - 1 - i] = ((MAGIC_CODE >> (8 * i)) & 0xFF) as u8;
    }

    std::fs::write("./testdisk.img", testdisk_data).expect("Error writing test disk data");
}
