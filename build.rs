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
    let mut cc = format!("{} cc -target {} -mcpu=baseline", quote(&zig), zig_target);
    if target.contains("apple") {
        let deployment = env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| {
            let output = Command::new(env::var_os("RUSTC").unwrap())
                .args(["--print", "deployment-target", "--target", &target])
                .output()
                .expect("query Rust's macOS deployment target");
            assert!(
                output.status.success(),
                "rustc could not report its deployment target"
            );
            String::from_utf8(output.stdout)
                .unwrap()
                .trim()
                .strip_prefix("MACOSX_DEPLOYMENT_TARGET=")
                .unwrap()
                .to_owned()
        });
        // Keep Zig, CMake and Rust on the same minimum macOS version.
        env::set_var("MACOSX_DEPLOYMENT_TARGET", &deployment);
        cc = format!(
            "{} cc -target {} -mcpu=baseline",
            quote(&zig),
            quote(&format!("{}.{}", zig_target, deployment))
        );
        let sdk = env::var("SDKROOT").unwrap_or_else(|_| {
            let output = Command::new("xcrun")
                .args(["--sdk", "macosx", "--show-sdk-path"])
                .output()
                .expect("install the Apple SDK or set SDKROOT for macOS targets");
            assert!(
                output.status.success(),
                "xcrun could not locate the macOS SDK"
            );
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        });
        // Zig supplies libc headers, but CommonCrypto and frameworks live in the SDK.
        cc.push_str(&format!(
            " -isysroot {} -isystem {} -iframework {}",
            quote(&sdk),
            quote(&format!("{}/usr/include", sdk)),
            quote(&format!("{}/System/Library/Frameworks", sdk))
        ));
    }
    for (path, command) in [(&compiler, cc), (&archiver, format!("{} ar", quote(&zig)))] {
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
    // musl includes these in libc, which Rust supplies. Asking a host GCC
    // linker for -lm would accidentally select its glibc libm.a.
    if !target.ends_with("musl") {
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=pthread");
        if target.contains("linux") {
            println!("cargo:rustc-link-lib=dl");
        }
    }
    if target.contains("apple") && env::var_os("CARGO_FEATURE_CURL").is_some() {
        println!("cargo:rustc-link-lib=framework=SystemConfiguration");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }
    println!("cargo:include={}/native/include", out.display());
    println!("cargo:root={}/native", out.display());
    println!("cargo:rerun-if-env-changed=ZIG");
    println!("cargo:rerun-if-env-changed=SDKROOT");
    println!("cargo:rerun-if-env-changed=MACOSX_DEPLOYMENT_TARGET");
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
