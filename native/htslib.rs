//! The HTSlib 1.19.1 source list, including its bundled CRAM codecs.
use std::{env, fs, path::Path, process::Command};

const SOURCES: &[&str] = &[
    "kfunc.c",
    "kstring.c",
    "bcf_sr_sort.c",
    "bgzf.c",
    "errmod.c",
    "faidx.c",
    "header.c",
    "hfile.c",
    "hts.c",
    "hts_expr.c",
    "hts_os.c",
    "md5.c",
    "multipart.c",
    "probaln.c",
    "realn.c",
    "regidx.c",
    "region.c",
    "sam.c",
    "sam_mods.c",
    "synced_bcf_reader.c",
    "vcf_sweep.c",
    "tbx.c",
    "textutils.c",
    "thread_pool.c",
    "vcf.c",
    "vcfutils.c",
    "cram/cram_codecs.c",
    "cram/cram_decode.c",
    "cram/cram_encode.c",
    "cram/cram_external.c",
    "cram/cram_index.c",
    "cram/cram_io.c",
    "cram/cram_stats.c",
    "cram/mFILE.c",
    "cram/open_trace_file.c",
    "cram/pooled_alloc.c",
    "cram/string_alloc.c",
    "htscodecs/htscodecs/arith_dynamic.c",
    "htscodecs/htscodecs/fqzcomp_qual.c",
    "htscodecs/htscodecs/htscodecs.c",
    "htscodecs/htscodecs/pack.c",
    "htscodecs/htscodecs/rANS_static4x16pr.c",
    "htscodecs/htscodecs/rANS_static32x16pr_avx2.c",
    "htscodecs/htscodecs/rANS_static32x16pr_avx512.c",
    "htscodecs/htscodecs/rANS_static32x16pr_sse4.c",
    "htscodecs/htscodecs/rANS_static32x16pr_neon.c",
    "htscodecs/htscodecs/rANS_static32x16pr.c",
    "htscodecs/htscodecs/rANS_static.c",
    "htscodecs/htscodecs/rle.c",
    "htscodecs/htscodecs/tokenise_name3.c",
    "htscodecs/htscodecs/utils.c",
];

pub fn build(out: &Path, compiler: &Path, archiver: &Path) {
    let source = Path::new("vendor/htslib");
    let build = out.join("htslib-build");
    fs::create_dir_all(&build).unwrap();
    let mut config = String::from("#define HAVE_DRAND48 1\n");
    let mut sources: Vec<_> = SOURCES.iter().map(|f| source.join(f)).collect();
    for (feature, definition, files) in [
        ("BZIP2", "HAVE_LIBBZ2", &[][..]),
        ("LZMA", "HAVE_LZMA_H", &[][..]),
        ("LIBDEFLATE", "HAVE_LIBDEFLATE", &[][..]),
        ("CURL", "HAVE_LIBCURL", &["hfile_libcurl.c"][..]),
        ("S3", "ENABLE_S3", &["hfile_s3.c", "hfile_s3_write.c"][..]),
        ("GCS", "ENABLE_GCS", &["hfile_gcs.c"][..]),
    ] {
        if env::var_os(format!("CARGO_FEATURE_{}", feature)).is_some() {
            config.push_str(&format!("#define {} 1\n", definition));
            sources.extend(files.iter().map(|f| source.join(f)));
        }
    }
    if env::var_os("CARGO_FEATURE_CURL").is_some() {
        config.push_str("#define HAVE_HMAC 1\n");
    }
    if env::var_os("CARGO_FEATURE_LZMA").is_some() {
        config.push_str("#define HAVE_LIBLZMA 1\n");
    }
    fs::write(build.join("config.h"), config).unwrap();
    fs::write(
        build.join("version.h"),
        "#define HTS_VERSION_TEXT \"1.19.1\"\n#define HTSCODECS_VERSION_TEXT \"1.6.0\"\n",
    )
    .unwrap();
    fs::write(build.join("config_vars.h"), "#define HTS_CC \"zig cc 0.15.2\"\n#define HTS_CPPFLAGS \"\"\n#define HTS_CFLAGS \"-O2 -fPIC -mcpu=baseline\"\n#define HTS_LDFLAGS \"\"\n#define HTS_LIBS \"vendored static libraries\"\n").unwrap();
    sources.push(Path::new("native/wrapper.c").to_owned());
    let mut objects = Vec::new();
    for (i, file) in sources.iter().enumerate() {
        let object = build.join(format!("{}.o", i));
        let status = Command::new(compiler)
            .args(["-c", "-O2", "-fPIC", "-Wno-deprecated-declarations"])
            .arg("-I")
            .arg(&build)
            .arg("-I")
            .arg(source)
            .arg("-I")
            .arg(out.join("native/include"))
            .arg(file)
            .arg("-o")
            .arg(&object)
            .status()
            .unwrap();
        assert!(status.success(), "failed compiling {}", file.display());
        objects.push(object);
    }
    let archive = out.join("native/lib/libhts.a");
    let _ = fs::remove_file(&archive);
    assert!(Command::new(archiver)
        .arg("crs")
        .arg(archive)
        .args(objects)
        .status()
        .unwrap()
        .success());
    let include = out.join("native/include/htslib");
    fs::create_dir_all(&include).unwrap();
    for entry in fs::read_dir(source.join("htslib")).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::copy(entry.path(), include.join(entry.file_name())).unwrap();
        }
    }
}
