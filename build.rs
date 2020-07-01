#[cfg(not(target_env = "msvc"))]
fn main() {
    println!("cargo:rerun-if-changed=src/SiUSBXp.c");

    let mut config = pkg_config::Config::new();
    config.print_system_libs(false);
    let mut gcc = cc::Build::new();

    match config.find("libusb") {
        Ok(lib) => lib.include_paths.iter().for_each(|include| {
            gcc.include(include);
        }),
        Err(e) => {
            panic!("run pkg_config fail: {:?}", e);
        }
    };

    // nowadays libusb of most OS use libusb-1.0 as backend
    match config.find("libusb-1.0") {
        Ok(lib) => lib.include_paths.iter().for_each(|include| {
            gcc.include(include);
        }),
        Err(_e) => {}
    };

    gcc.file("src/SiUSBXp.c")
        .flag("-Wno-unused-parameter")
        .pic(true)
        .compile("SiUSBXp");
}

#[cfg(target_env = "msvc")]
fn main() {
    println!("cargo:rerun-if-changed=src/SiUSBXp.c");
    if std::env::var_os("VCPKGRS_DYNAMIC").is_none() {
        std::env::set_var("VCPKGRS_DYNAMIC", "1");
    }

    let lib = vcpkg::Config::new()
        .emit_includes(true)
        .find_package("libusb-win32");

    if let Err(e) = lib {
        panic!("note: vcpkg did not find libusb-win32: {}", e);
    }
    let lib = lib.unwrap();

    let mut gcc = cc::Build::new();
    for include in lib.include_paths.iter() {
        gcc.include(include);
    }

    gcc.file("src/SiUSBXp.c")
        // ignore unused parameter
        .flag("/wd4512")
        .pic(true)
        .compile("SiUSBXp");
}
