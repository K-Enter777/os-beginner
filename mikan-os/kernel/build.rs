use std::{
    io,
    path::{Path, PathBuf},
};

fn main() -> io::Result<()> {
    let files = get_cpp_files("src/usb")?;
    cc::Build::new()
        .cpp(true)
        .compiler("clang++")
        .no_default_flags(true)
        .cpp_set_stdlib("c++")
        .include("src")
        .include("src/cxx")
        .include("src/usb")
        .include("src/usb/xhci")
        .include("../../devenv/x86_64-elf/include")
        .include("../../devenv/x86_64-elf/include/c++/v1")
        .opt_level(2)
        .extra_warnings(false)
        .target("x86_64-elf")
        .flag_if_supported("-ffreestanding")
        .flag_if_supported("-mno-red-zone")
        .flag_if_supported("-fno-exceptions")
        .flag_if_supported("-fno-rtti")
        .std("c++17")
        .files(files)
        .file("src/cxx/logger.cpp")
        .file("src/cxx/newlib_support.c")
        .file("src/cxx/libcxx_support.cpp")
        .compile("usb");

    println!("cargo:rerun-if-changed=src/*");
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}

fn get_cpp_files(dir_path: &str) -> io::Result<Vec<PathBuf>> {
    let mut ret = vec![];

    let files = std::fs::read_dir(dir_path)?;

    for entry in files {
        let file = entry?;
        if file.file_type()?.is_dir() {
            let dir_path = Path::new(dir_path).join(file.file_name().to_str().unwrap());
            ret.append(&mut get_cpp_files(dir_path.to_str().unwrap())?);
        } else if file.file_type()?.is_file() {
            if file.file_name().to_str().unwrap().ends_with(".cpp") {
                ret.push(file.path());
            }
        }
    }
    Ok(ret)
}
