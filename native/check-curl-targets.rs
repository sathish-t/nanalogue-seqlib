//! Maintainer-only compile/link checks using the shared artifact model.
//! rustc --edition=2018 native/check-curl-targets.rs -o target/check-curl-targets
//! target/check-curl-targets [substring of Zig target]
#[allow(dead_code)]
#[path = "target_spec.rs"]
mod target_spec;
use std::{env, fs, path::PathBuf, process::Command};
use target_spec::{Libc, Os, Target, ARTIFACTS};

fn run(command: &mut Command) {
    eprintln!("{command:?}");
    assert!(command.status().expect("execute command").success());
}

fn main() {
    let zig = env::var("ZIG").unwrap_or_else(|_| "zig".into());
    let filter = env::args().nth(1).unwrap_or_default();
    for &(rust, baseline) in ARTIFACTS {
        let t = Target::parse(rust);
        let target = t.artifact_target(baseline);
        if t.os != Os::Linux || !target.contains(&filter) {
            continue;
        }
        let prefix = PathBuf::from("target/curl-matrix").join(&target);
        fs::create_dir_all(&prefix).unwrap();
        for step in ["compression", "openssl", "curl", "htslib"] {
            run(Command::new(&zig)
                .args(["build", step, "-j2", "--prefix"])
                .arg(&prefix)
                .arg(format!("-Dtarget={target}"))
                .arg(format!("-Dcpu={}", t.cpu()))
                .args([
                    "-Dbzip2=true",
                    "-Dlzma=true",
                    "-Dlibdeflate=true",
                    "-Dcurl=true",
                    "-Ds3=true",
                    "-Dgcs=true",
                ]));
        }
        let executable = prefix.join("curl-test");
        let mut cc = Command::new(&zig);
        cc.args(["cc", "-target", &target])
            .arg(format!("-mcpu={}", t.cpu()))
            .args([
                "-O2",
                "-fsanitize=undefined",
                "-fsanitize-trap=undefined",
                "-D_GNU_SOURCE",
                "-DCHECK_HTSLIB",
                "-Inative",
            ])
            .arg(format!("-I{}", prefix.join("include").display()))
            .arg("native/curl-test.c");
        if t.libc == Libc::Gnu {
            cc.arg("-DNANALOGUE_CURL_GNU");
        }
        for lib in [
            "hts", "curl", "ssl", "crypto", "deflate", "lzma", "bz2", "z",
        ] {
            cc.arg(prefix.join(format!("lib/lib{lib}.a")));
        }
        run(cc.args(["-pthread", "-ldl", "-lm", "-o"]).arg(&executable));
        if target.starts_with("x86_64-") && cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            run(&mut Command::new(&executable));
        }
        println!(
            "PASS {target} CPU={} (compile/link; runtime only for host x86_64 Linux)",
            t.cpu()
        );
    }
}
