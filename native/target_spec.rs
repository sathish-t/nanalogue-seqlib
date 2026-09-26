//! Shared native target specifications. Artifact ABI floors are not CI runner names.
//! This module has no Cargo dependencies so the maintainer tools can reuse it.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Linux,
    Macos,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Aarch64,
    Armv7,
    Armv6,
    Powerpc64le,
    Riscv64gc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Libc {
    Gnu,
    Musl,
    Darwin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Baseline {
    Macos,
    Manylinux(u8), // glibc 2.<minor>; not the build host's glibc
    Musllinux12,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub os: Os,
    pub arch: Arch,
    pub libc: Libc,
}

impl Target {
    pub fn parse(rust: &str) -> Self {
        let (os, arch, libc) = match rust {
            "x86_64-unknown-linux-gnu" => (Os::Linux, Arch::X86_64, Libc::Gnu),
            "aarch64-unknown-linux-gnu" => (Os::Linux, Arch::Aarch64, Libc::Gnu),
            "armv7-unknown-linux-gnueabihf" => (Os::Linux, Arch::Armv7, Libc::Gnu),
            "powerpc64le-unknown-linux-gnu" => (Os::Linux, Arch::Powerpc64le, Libc::Gnu),
            "riscv64gc-unknown-linux-gnu" => (Os::Linux, Arch::Riscv64gc, Libc::Gnu),
            "x86_64-unknown-linux-musl" => (Os::Linux, Arch::X86_64, Libc::Musl),
            "aarch64-unknown-linux-musl" => (Os::Linux, Arch::Aarch64, Libc::Musl),
            "arm-unknown-linux-musleabihf" => (Os::Linux, Arch::Armv6, Libc::Musl),
            "powerpc64le-unknown-linux-musl" => (Os::Linux, Arch::Powerpc64le, Libc::Musl),
            "x86_64-apple-darwin" => (Os::Macos, Arch::X86_64, Libc::Darwin),
            "aarch64-apple-darwin" => (Os::Macos, Arch::Aarch64, Libc::Darwin),
            _ => panic!("unsupported vendored HTSlib target: {}", rust),
        };
        Self { os, arch, libc }
    }

    pub fn zig(self) -> String {
        let arch = match self.arch {
            Arch::X86_64 => "x86_64",
            Arch::Aarch64 => "aarch64",
            Arch::Armv7 | Arch::Armv6 => "arm",
            Arch::Powerpc64le => "powerpc64le",
            Arch::Riscv64gc => "riscv64",
        };
        let abi = match (self.libc, self.arm_hard_float()) {
            (Libc::Gnu, true) => "linux-gnueabihf",
            (Libc::Musl, true) => "linux-musleabihf",
            (Libc::Gnu, false) => "linux-gnu",
            (Libc::Musl, false) => "linux-musl",
            (Libc::Darwin, _) => "macos",
        };
        format!("{}-{}", arch, abi)
    }

    pub fn cpu(self) -> &'static str {
        match self.arch {
            Arch::Armv7 => "generic+v7a+vfp3d16",
            Arch::Armv6 => "generic+v6+vfp2",
            // Rust's gc ABI requires double-float, atomics, compressed and
            // multiply/divide; plain Zig baseline is not the Rust gc contract.
            Arch::Riscv64gc => "baseline+m+a+f+d+c+zicsr+zifencei",
            _ => "baseline",
        }
    }

    pub fn pointer_bits(self) -> u8 {
        if self.arm_hard_float() {
            32
        } else {
            64
        }
    }

    pub fn arm_hard_float(self) -> bool {
        matches!(self.arch, Arch::Armv6 | Arch::Armv7)
    }

    pub fn little_endian(self) -> bool {
        // All architectures in the historical artifact matrix are LE,
        // including powerpc64le. Do not infer byte order from pointer width.
        true
    }

    pub fn openssl(self) -> &'static str {
        match (self.os, self.arch) {
            (Os::Macos, Arch::X86_64) => "darwin64-x86_64-cc",
            (Os::Macos, Arch::Aarch64) => "darwin64-arm64-cc",
            (Os::Linux, Arch::X86_64) => "linux-x86_64",
            (Os::Linux, Arch::Aarch64) => "linux-aarch64",
            (Os::Linux, Arch::Armv6 | Arch::Armv7) => "linux-armv4",
            (Os::Linux, Arch::Powerpc64le) => "linux-ppc64le",
            (Os::Linux, Arch::Riscv64gc) => "linux64-riscv64",
            _ => panic!("invalid OS/architecture combination"),
        }
    }

    pub fn has_bindings(self) -> bool {
        matches!(self.arch, Arch::X86_64 | Arch::Aarch64)
    }

    pub fn artifact_target(self, baseline: Baseline) -> String {
        match (self.libc, baseline) {
            (Libc::Gnu, Baseline::Manylinux(minor)) => {
                let supported = match self.arch {
                    Arch::X86_64 | Arch::Aarch64 => matches!(minor, 17 | 28 | 34),
                    Arch::Armv7 => minor == 17,
                    Arch::Powerpc64le | Arch::Riscv64gc => minor == 29,
                    _ => false,
                };
                assert!(supported, "not a historical manylinux artifact");
                format!("{}.2.{}", self.zig(), minor)
            }
            (Libc::Musl, Baseline::Musllinux12) => self.zig(),
            // The deployment version is supplied by Rust/SDK, not runner OS.
            (Libc::Darwin, Baseline::Macos) => self.zig(),
            _ => panic!("artifact baseline does not match target libc"),
        }
    }
}

/// Fifteen artifacts, eleven Rust triples, seven OpenSSL configurations.
/// The unsuffixed historical musllinux `arm` is modeled as ARMv6 hard-float.
pub const ARTIFACTS: &[(&str, Baseline)] = &[
    ("x86_64-apple-darwin", Baseline::Macos),
    ("aarch64-apple-darwin", Baseline::Macos),
    ("x86_64-unknown-linux-gnu", Baseline::Manylinux(17)),
    ("aarch64-unknown-linux-gnu", Baseline::Manylinux(17)),
    ("armv7-unknown-linux-gnueabihf", Baseline::Manylinux(17)),
    ("x86_64-unknown-linux-gnu", Baseline::Manylinux(28)),
    ("aarch64-unknown-linux-gnu", Baseline::Manylinux(28)),
    ("powerpc64le-unknown-linux-gnu", Baseline::Manylinux(29)),
    ("riscv64gc-unknown-linux-gnu", Baseline::Manylinux(29)),
    ("x86_64-unknown-linux-gnu", Baseline::Manylinux(34)),
    ("aarch64-unknown-linux-gnu", Baseline::Manylinux(34)),
    ("x86_64-unknown-linux-musl", Baseline::Musllinux12),
    ("aarch64-unknown-linux-musl", Baseline::Musllinux12),
    ("arm-unknown-linux-musleabihf", Baseline::Musllinux12),
    ("powerpc64le-unknown-linux-musl", Baseline::Musllinux12),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn historical_matrix() {
        assert_eq!(ARTIFACTS.len(), 15);
        let triples: BTreeSet<_> = ARTIFACTS.iter().map(|(t, _)| t).collect();
        assert_eq!(triples.len(), 11);
        let mut configs = BTreeSet::new();
        for &(rust, baseline) in ARTIFACTS {
            let target = Target::parse(rust);
            assert!(target.little_endian());
            assert!(!target.artifact_target(baseline).is_empty());
            configs.insert(target.openssl());
        }
        assert_eq!(configs.len(), 7);
    }

    #[test]
    fn abi_and_cpu_are_not_runner_properties() {
        let arm = Target::parse("armv7-unknown-linux-gnueabihf");
        assert_eq!(
            arm.artifact_target(Baseline::Manylinux(17)),
            "arm-linux-gnueabihf.2.17"
        );
        assert_eq!(arm.pointer_bits(), 32);
        assert!(arm.arm_hard_float());
        assert!(!arm.has_bindings());
        let ppc = Target::parse("powerpc64le-unknown-linux-musl");
        assert_eq!(ppc.pointer_bits(), 64);
        assert_eq!(ppc.cpu(), "baseline");
        assert!(!ppc.arm_hard_float());
        let rv = Target::parse("riscv64gc-unknown-linux-gnu");
        assert_eq!(rv.cpu(), "baseline+m+a+f+d+c+zicsr+zifencei");
        let x86 = Target::parse("x86_64-unknown-linux-gnu");
        assert_eq!(
            x86.artifact_target(Baseline::Manylinux(34)),
            "x86_64-linux-gnu.2.34"
        );
        assert_eq!(x86.cpu(), "baseline");
    }

    #[test]
    #[ignore = "requires Zig 0.15.2; run with --include-ignored"]
    fn zig_preprocessor_agrees_with_all_artifact_abis() {
        for &(rust, baseline) in ARTIFACTS {
            let target = Target::parse(rust);
            let result = std::process::Command::new(std::env::var("ZIG").unwrap_or("zig".into()))
                .args(["cc", "-target", &target.artifact_target(baseline)])
                .arg(format!("-mcpu={}", target.cpu()))
                .args(["-dM", "-E", "-x", "c", "/dev/null"])
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}: {}",
                rust,
                String::from_utf8_lossy(&result.stderr)
            );
            let macros = String::from_utf8(result.stdout).unwrap();
            let has = |line: &str| macros.lines().any(|m| m == line);
            assert!(
                has(&format!(
                    "#define __SIZEOF_POINTER__ {}",
                    target.pointer_bits() / 8
                )),
                "{} pointer width",
                rust
            );
            assert!(
                has("#define __BYTE_ORDER__ __ORDER_LITTLE_ENDIAN__"),
                "{} byte order",
                rust
            );
            if target.arm_hard_float() {
                assert!(has("#define __ARM_PCS_VFP 1"), "{} float ABI", rust);
                let version = if target.arch == Arch::Armv7 { 7 } else { 6 };
                assert!(
                    has(&format!("#define __ARM_ARCH {}", version)),
                    "{} ARM version",
                    rust
                );
            }
            if target.arch == Arch::Riscv64gc {
                for feature in [
                    "#define __riscv_float_abi_double 1",
                    "#define __riscv_atomic 1",
                    "#define __riscv_compressed 1",
                    "#define __riscv_mul 1",
                ] {
                    assert!(has(feature), "{} missing {}", rust, feature);
                }
            }
        }
    }

    #[test]
    #[should_panic(expected = "artifact baseline does not match target libc")]
    fn cannot_apply_glibc_version_to_musl() {
        Target::parse("x86_64-unknown-linux-musl").artifact_target(Baseline::Manylinux(17));
    }

    #[test]
    #[should_panic(expected = "not a historical manylinux artifact")]
    fn cannot_use_riscv_below_its_artifact_floor() {
        Target::parse("riscv64gc-unknown-linux-gnu").artifact_target(Baseline::Manylinux(17));
    }
}
