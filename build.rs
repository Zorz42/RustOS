#[allow(clippy::expect_used)]
fn main() {
    //println!("cargo::rerun-if-changed=testdisk.img");

    const NUM_SECTORS: usize = 10000;

    let testdisk_data = vec![0u8; NUM_SECTORS * 512];

    std::fs::write("./testdisk.img", testdisk_data).expect("Error writing test disk data");
}
