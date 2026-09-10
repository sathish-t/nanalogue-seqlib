use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.zig");
    println!("cargo:rerun-if-changed=native/fai.zig");
    println!("cargo:rerun-if-changed=vendor/htslib");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must set OUT_DIR"));
    let prefix = out_dir.join("zig");
    let zig = env::var_os("ZIG").unwrap_or_else(|| "zig".into());

    let status = Command::new(zig)
        .args(["build", "--prefix"])
        .arg(&prefix)
        .status()
        .expect("failed to run Zig; install Zig or set the ZIG environment variable")
        .success();
    assert!(status, "Zig failed to build the HTSlib FAI facade");

    println!(
        "cargo:rustc-link-search=native={}",
        prefix.join("lib").display()
    );
    println!("cargo:rustc-link-search=native=vendor/htslib");
    println!("cargo:rustc-link-lib=static=nanalogue_fai");
    println!("cargo:rustc-link-lib=static=hts");
    println!("cargo:rustc-link-lib=z");
}
