//! Builds the vendored compression libraries without consulting the host.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn build(out: &Path, target: &str, compiler: &Path, archiver: &Path) {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let prefix = out.join("native");
    let lib = prefix.join("lib");
    let include = prefix.join("include");
    fs::create_dir_all(&lib).unwrap();
    fs::create_dir_all(&include).unwrap();

    let zlib_flags = if target.contains("windows") {
        &[][..]
    } else {
        &["-DHAVE_UNISTD_H=1"][..]
    };
    compile(
        &root.join("vendor/zlib"),
        &out.join("compression-zlib"),
        (compiler, archiver),
        &lib.join("libz.a"),
        &zlib_sources(),
        &["."],
        zlib_flags,
    );
    copy(&root.join("vendor/zlib/zlib.h"), &include.join("zlib.h"));
    copy(&root.join("vendor/zlib/zconf.h"), &include.join("zconf.h"));

    if feature("BZIP2") {
        compile(
            &root.join("vendor/bzip2"),
            &out.join("compression-bzip2"),
            (compiler, archiver),
            &lib.join("libbz2.a"),
            &names(&[
                "blocksort.c",
                "huffman.c",
                "crctable.c",
                "randtable.c",
                "compress.c",
                "decompress.c",
                "bzlib.c",
            ]),
            &["."],
            &[],
        );
        copy(&root.join("vendor/bzip2/bzlib.h"), &include.join("bzlib.h"));
    }
    if feature("LZMA") {
        let source = root.join("vendor/xz");
        let mut sources = Vec::new();
        for dir in [
            "src/liblzma/common",
            "src/liblzma/lzma",
            "src/liblzma/lz",
            "src/liblzma/check",
            "src/liblzma/delta",
            "src/liblzma/rangecoder",
            "src/liblzma/simple",
        ] {
            for entry in fs::read_dir(source.join(dir)).unwrap() {
                let path = entry.unwrap().path();
                let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or("");
                if path.extension() == Some(OsStr::new("c"))
                    && stem != "crc32_small"
                    && stem != "crc64_small"
                    && !stem.ends_with("tablegen")
                {
                    sources.push(path.strip_prefix(&source).unwrap().to_owned());
                }
            }
        }
        sources.extend(names(&[
            "src/common/tuklib_cpucores.c",
            "src/common/tuklib_physmem.c",
        ]));
        sources.sort();
        let dirs = [
            ".",
            "src/liblzma/api",
            "src/liblzma/lzma",
            "src/liblzma/lz",
            "src/liblzma/check",
            "src/liblzma/simple",
            "src/liblzma/delta",
            "src/liblzma/common",
            "src/liblzma/rangecoder",
            "src/common",
        ];
        compile(
            &source,
            &out.join("compression-xz"),
            (compiler, archiver),
            &lib.join("liblzma.a"),
            &sources,
            &dirs,
            &["-DHAVE_CONFIG_H=1", "-std=c99"],
        );
        copy(
            &source.join("src/liblzma/api/lzma.h"),
            &include.join("lzma.h"),
        );
        copy_dir(&source.join("src/liblzma/api/lzma"), &include.join("lzma"));
    }
    if feature("LIBDEFLATE") {
        let sources = names(&[
            "lib/utils.c",
            "lib/arm/cpu_features.c",
            "lib/x86/cpu_features.c",
            "lib/deflate_compress.c",
            "lib/deflate_decompress.c",
            "lib/adler32.c",
            "lib/zlib_compress.c",
            "lib/zlib_decompress.c",
            "lib/crc32.c",
            "lib/gzip_compress.c",
            "lib/gzip_decompress.c",
        ]);
        compile(
            &root.join("vendor/libdeflate"),
            &out.join("compression-libdeflate"),
            (compiler, archiver),
            &lib.join("libdeflate.a"),
            &sources,
            &[".", "lib"],
            &[],
        );
        copy(
            &root.join("vendor/libdeflate/libdeflate.h"),
            &include.join("libdeflate.h"),
        );
    }
}

fn feature(name: &str) -> bool {
    env::var_os(format!("CARGO_FEATURE_{name}")).is_some()
}
fn names(xs: &[&str]) -> Vec<PathBuf> {
    xs.iter().map(PathBuf::from).collect()
}
fn zlib_sources() -> Vec<PathBuf> {
    names(&[
        "adler32.c",
        "compress.c",
        "crc32.c",
        "deflate.c",
        "gzclose.c",
        "gzlib.c",
        "gzread.c",
        "gzwrite.c",
        "infback.c",
        "inffast.c",
        "inflate.c",
        "inftrees.c",
        "trees.c",
        "uncompr.c",
        "zutil.c",
    ])
}

fn compile(
    source: &Path,
    build: &Path,
    (cc, ar): (&Path, &Path),
    archive: &Path,
    sources: &[PathBuf],
    includes: &[&str],
    flags: &[&str],
) {
    if build.exists() {
        fs::remove_dir_all(build).unwrap();
    }
    fs::create_dir_all(build).unwrap();
    let mut objects = Vec::new();
    for (i, file) in sources.iter().enumerate() {
        let object = build.join(format!("{i}.o"));
        let mut cmd = Command::new(cc);
        cmd.arg("-c").arg("-O2").arg("-fPIC").args(flags);
        for dir in includes {
            cmd.arg("-I").arg(source.join(dir));
        }
        cmd.arg(source.join(file)).arg("-o").arg(&object);
        run(&mut cmd);
        objects.push(object);
    }
    let _ = fs::remove_file(archive);
    let mut cmd = Command::new(ar);
    cmd.arg("crs").arg(archive).args(&objects);
    run(&mut cmd);
}

fn run(cmd: &mut Command) {
    let shown = format!("{cmd:?}");
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to run {}: {}", shown, e));
    assert!(status.success(), "command failed ({}): {}", status, shown);
}
fn copy(from: &Path, to: &Path) {
    fs::copy(from, to).unwrap_or_else(|e| panic!("copy {}: {e}", from.display()));
}
fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy(&entry.path(), &to.join(entry.file_name()));
        }
    }
}
