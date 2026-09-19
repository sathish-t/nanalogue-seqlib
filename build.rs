#[path = "native/compression.rs"]
mod compression;
#[path = "native/htslib.rs"]
mod htslib;
#[path = "native/network.rs"]
mod network;

use std::{env, fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

fn main() {
    let target = env::var("TARGET").unwrap();
    let zig_target = match target.as_str() {
        "x86_64-unknown-linux-gnu" => "x86_64-linux-gnu",
        "aarch64-unknown-linux-gnu" => "aarch64-linux-gnu",
        "x86_64-unknown-linux-musl" => "x86_64-linux-musl",
        "aarch64-unknown-linux-musl" => "aarch64-linux-musl",
        "x86_64-apple-darwin" => "x86_64-macos",
        "aarch64-apple-darwin" => "aarch64-macos",
        _ => panic!("unsupported vendored HTSlib target: {}", target),
    };
    let zig = env::var("ZIG").unwrap_or_else(|_| "zig".into());
    let version = Command::new(&zig)
        .arg("version")
        .output()
        .expect("install Zig 0.14.1 and put zig on PATH (or set ZIG to its absolute path)");
    assert!(
        version.status.success() && version.stdout == b"0.14.1\n",
        "this native build requires Zig 0.14.1"
    );
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let compiler = out.join("zig-cc.sh");
    let archiver = out.join("zig-ar.sh");
    let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    for (path, command) in [
        (
            &compiler,
            format!("{} cc -target {} -mcpu=baseline", quote(&zig), zig_target),
        ),
        (&archiver, format!("{} ar", quote(&zig))),
    ] {
        fs::write(path, format!("#!/bin/sh\nexec {} \"$@\"\n", command)).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::copy(
        format!("native/bindings/{}.rs", target),
        out.join("bindings.rs"),
    )
    .expect("missing checked-in bindings for supported target");
    compression::build(&out, &target, &compiler, &archiver);
    network::build(&out, &target, &compiler, &archiver);
    htslib::build(&out, &compiler, &archiver);
    println!(
        "cargo:rustc-link-search=native={}/native/lib",
        out.display()
    );
    println!("cargo:rustc-link-lib=static=hts");
    for (feature, libraries) in [
        ("CURL", &["curl", "ssl", "crypto"][..]),
        ("LIBDEFLATE", &["deflate"][..]),
        ("LZMA", &["lzma"][..]),
        ("BZIP2", &["bz2"][..]),
    ] {
        if env::var_os(format!("CARGO_FEATURE_{}", feature)).is_some() {
            for library in libraries {
                println!("cargo:rustc-link-lib=static={}", library);
            }
        }
    }
    println!("cargo:rustc-link-lib=static=z");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=pthread");
    if target.contains("linux") {
        println!("cargo:rustc-link-lib=dl");
    }
    println!("cargo:include={}/native/include", out.display());
    println!("cargo:root={}/native", out.display());
    println!("cargo:rerun-if-env-changed=ZIG");
    for file in [
        "compression.rs",
        "network.rs",
        "htslib.rs",
        "wrapper.c",
        "wrapper.h",
    ] {
        println!("cargo:rerun-if-changed=native/{}", file);
    }
    println!("cargo:rerun-if-changed=native/bindings/{}.rs", target);
    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-changed=build.rs");
}
