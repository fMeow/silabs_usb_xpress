fn main() {
    cc::Build::new()
        .file("src/SiUSBXp.c")
        .flag("-Wno-unused-parameter")
        .pic(true)
        .compile("SiUSBXp");
    println!("cargo:rerun-if-changed=src/SiUSBXp.c");
    pkg_config::find_library("libusb").unwrap();
}
