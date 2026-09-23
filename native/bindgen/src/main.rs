//! Maintainer-only binding generator. Run from the repository root.
use std::{env, path::PathBuf, process::Command};

#[allow(dead_code)]
#[path = "../../target_spec.rs"]
mod native_target;

fn main() {
    let target = env::args().nth(1).expect("Rust target argument");
    let descriptor = native_target::Target::parse(&target);
    let zig_target = descriptor.zig();
    let cpu = format!("-mcpu={}", descriptor.cpu());
    let preprocessed = PathBuf::from("target").join(format!("htslib-{target}.i"));
    std::fs::create_dir_all("target").unwrap();
    assert!(Command::new(env::var("ZIG").unwrap_or("zig".into()))
        .args([
            "cc",
            "-target",
            &zig_target,
            &cpu,
            "-E",
            "-dD",
            "-C",
            "-Ivendor/htslib",
            "native/wrapper.h",
            "-o"
        ])
        .arg(&preprocessed)
        .status()
        .unwrap()
        .success());
    bindgen::Builder::default()
        .header(preprocessed.to_str().unwrap())
        .clang_arg(format!("--target={target}"))
        .layout_tests(true)
        .generate_comments(false)
        // Use Rust/libc for these runtime intrinsics, not expanded C typedefs.
        .blocklist_function("memcpy|memmove|memset|memcmp|strlen|bcmp")
        .blocklist_function("strtold")
        .blocklist_type("max_align_t")
        .generate()
        .expect("generate target bindings")
        .write_to_file(format!("native/bindings/{target}.rs"))
        .unwrap();
}
