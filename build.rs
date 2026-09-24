#[allow(dead_code)] // Artifact metadata is also used by maintainer-only tools.
#[path = "native/target_spec.rs"]
mod native_target;
#[path = "native/network.rs"]
mod network;

use std::{
    env, fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    let target = env::var("TARGET").unwrap();
    let descriptor = native_target::Target::parse(&target);
    assert!(descriptor.has_bindings(), "target {} is modeled but not enabled: checked-in bindings and native ABI validation are still required", target);
    let mut zig_target = descriptor.zig();
    let zig = env::var("ZIG").unwrap_or_else(|_| "zig".into());
    let version = Command::new(&zig)
        .arg("version")
        .output()
        .expect("install Zig 0.15.2 and put zig on PATH (or set ZIG to its absolute path)");
    assert!(
        version.status.success() && version.stdout == b"0.15.2\n",
        "this native build requires Zig 0.15.2"
    );
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let compiler = out.join("zig-cc.sh");
    let archiver = out.join("zig-ar.sh");
    let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    let mut cc = format!(
        "{} cc -target {} -mcpu={}",
        quote(&zig),
        zig_target,
        descriptor.cpu()
    );
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
        zig_target = format!("{}.{}", zig_target, deployment);
        cc = format!(
            "{} cc -target {} -mcpu={}",
            quote(&zig),
            quote(&zig_target),
            descriptor.cpu()
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
        // Direct Zig OpenSSL and curl must use the same SDK as this wrapper.
        env::set_var("SDKROOT", &sdk);
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
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    run_zig(
        &root,
        &out,
        &zig,
        &zig_target,
        descriptor.cpu(),
        "compression",
    );
    if env::var_os("CARGO_FEATURE_CURL").is_some() {
        run_zig(&root, &out, &zig, &zig_target, descriptor.cpu(), "openssl");
    }
    network::build(&out, descriptor, &compiler, &archiver);
    run_zig(&root, &out, &zig, &zig_target, descriptor.cpu(), "htslib");
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
    if descriptor.libc != native_target::Libc::Musl {
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=pthread");
        if target.contains("linux") {
            println!("cargo:rustc-link-lib=dl");
        }
    }
    if target.contains("apple") && env::var_os("CARGO_FEATURE_CURL").is_some() {
        println!("cargo:rustc-link-lib=framework=SystemConfiguration");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreServices");
        println!("cargo:rustc-link-lib=framework=Security");
    }
    println!("cargo:include={}/native/include", out.display());
    println!("cargo:root={}/native", out.display());
    println!("cargo:rerun-if-env-changed=ZIG");
    println!("cargo:rerun-if-env-changed=SDKROOT");
    println!("cargo:rerun-if-env-changed=MACOSX_DEPLOYMENT_TARGET");
    for file in ["network.rs", "target_spec.rs", "wrapper.c", "wrapper.h"] {
        println!("cargo:rerun-if-changed=native/{}", file);
    }
    println!("cargo:rerun-if-changed=native/bindings/{}.rs", target);
    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-changed=build.zig");
    println!("cargo:rerun-if-changed=native/openssl.zig");
    println!("cargo:rerun-if-changed=native/openssl");
    println!("cargo:rerun-if-changed=build.rs");
}

fn run_zig(root: &Path, out: &Path, zig: &str, target: &str, cpu: &str, step: &str) {
    let mut command = Command::new(zig);
    command
        .current_dir(root)
        .arg("build")
        .arg(step)
        .arg("--prefix")
        .arg(out.join("native"))
        .arg("--cache-dir")
        .arg(out.join("zig-cache"))
        .arg("--global-cache-dir")
        .arg(out.join("zig-global-cache"))
        .arg(format!(
            "-j{}",
            env::var("NUM_JOBS").unwrap_or_else(|_| "1".into())
        ))
        .arg(format!("-Dtarget={target}"))
        .arg(format!("-Dcpu={cpu}"));
    for feature in ["bzip2", "lzma", "libdeflate", "curl", "s3", "gcs"] {
        command.arg(format!(
            "-D{feature}={}",
            env::var_os(format!("CARGO_FEATURE_{}", feature.to_uppercase())).is_some()
        ));
    }
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("failed to execute Zig {} build: {}", step, error));
    assert!(
        status.success(),
        "Zig {} build failed with {}",
        step,
        status
    );
}
