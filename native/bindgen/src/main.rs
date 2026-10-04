//! Maintainer-only binding generator. Run from the repository root.
use std::{env, path::PathBuf, process::Command};

fn main() {
    let target = env::args().nth(1).expect("Rust target argument");
    let zig_target = match target.as_str() {
        "wasm32-wasip1" => "wasm32-wasi",
        "x86_64-unknown-linux-gnu" => "x86_64-linux-gnu",
        "aarch64-unknown-linux-gnu" => "aarch64-linux-gnu",
        "x86_64-unknown-linux-musl" => "x86_64-linux-musl",
        "aarch64-unknown-linux-musl" => "aarch64-linux-musl",
        "x86_64-apple-darwin" => "x86_64-macos",
        "aarch64-apple-darwin" => "aarch64-macos",
        _ => panic!("unsupported target"),
    };
    let wasi_flags: &[&str] = if target == "wasm32-wasip1" {
        &[
            "-D_WASI_EMULATED_SIGNAL",
            "-D_WASI_EMULATED_MMAN",
            "-D_WASI_EMULATED_GETPID",
            "-D_WASI_EMULATED_PROCESS_CLOCKS",
        ]
    } else {
        &[]
    };
    let preprocessed = PathBuf::from("target").join(format!("htslib-{target}.i"));
    std::fs::create_dir_all("target").unwrap();
    assert!(Command::new(env::var("ZIG").unwrap_or("zig".into()))
        .args([
            "cc",
            "-target",
            zig_target,
            "-mcpu=baseline",
            "-E",
            "-dD",
            "-C",
            "-Ivendor/htslib",
        ])
        .args(wasi_flags)
        .args(["native/wrapper.h", "-o"])
        .arg(&preprocessed)
        .status()
        .unwrap()
        .success());
    bindgen::Builder::default()
        .header(preprocessed.to_str().unwrap())
        .clang_arg(format!(
            "--target={}",
            if target == "wasm32-wasip1" {
                "wasm32-wasi"
            } else {
                &target
            }
        ))
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
