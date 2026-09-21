const std = @import("std");

const zlib_sources = &.{
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
};

const bzip2_sources = &.{
    "blocksort.c",
    "huffman.c",
    "crctable.c",
    "randtable.c",
    "compress.c",
    "decompress.c",
    "bzlib.c",
};

const lzma_sources = &.{
    "src/common/tuklib_cpucores.c",
    "src/common/tuklib_physmem.c",
    "src/liblzma/check/check.c",
    "src/liblzma/check/crc32_fast.c",
    "src/liblzma/check/crc64_fast.c",
    "src/liblzma/check/sha256.c",
    "src/liblzma/common/alone_decoder.c",
    "src/liblzma/common/alone_encoder.c",
    "src/liblzma/common/auto_decoder.c",
    "src/liblzma/common/block_buffer_decoder.c",
    "src/liblzma/common/block_buffer_encoder.c",
    "src/liblzma/common/block_decoder.c",
    "src/liblzma/common/block_encoder.c",
    "src/liblzma/common/block_header_decoder.c",
    "src/liblzma/common/block_header_encoder.c",
    "src/liblzma/common/block_util.c",
    "src/liblzma/common/common.c",
    "src/liblzma/common/easy_buffer_encoder.c",
    "src/liblzma/common/easy_decoder_memusage.c",
    "src/liblzma/common/easy_encoder.c",
    "src/liblzma/common/easy_encoder_memusage.c",
    "src/liblzma/common/easy_preset.c",
    "src/liblzma/common/file_info.c",
    "src/liblzma/common/filter_buffer_decoder.c",
    "src/liblzma/common/filter_buffer_encoder.c",
    "src/liblzma/common/filter_common.c",
    "src/liblzma/common/filter_decoder.c",
    "src/liblzma/common/filter_encoder.c",
    "src/liblzma/common/filter_flags_decoder.c",
    "src/liblzma/common/filter_flags_encoder.c",
    "src/liblzma/common/hardware_cputhreads.c",
    "src/liblzma/common/hardware_physmem.c",
    "src/liblzma/common/index.c",
    "src/liblzma/common/index_decoder.c",
    "src/liblzma/common/index_encoder.c",
    "src/liblzma/common/index_hash.c",
    "src/liblzma/common/lzip_decoder.c",
    "src/liblzma/common/microlzma_decoder.c",
    "src/liblzma/common/microlzma_encoder.c",
    "src/liblzma/common/outqueue.c",
    "src/liblzma/common/stream_buffer_decoder.c",
    "src/liblzma/common/stream_buffer_encoder.c",
    "src/liblzma/common/stream_decoder.c",
    "src/liblzma/common/stream_decoder_mt.c",
    "src/liblzma/common/stream_encoder.c",
    "src/liblzma/common/stream_encoder_mt.c",
    "src/liblzma/common/stream_flags_common.c",
    "src/liblzma/common/stream_flags_decoder.c",
    "src/liblzma/common/stream_flags_encoder.c",
    "src/liblzma/common/string_conversion.c",
    "src/liblzma/common/vli_decoder.c",
    "src/liblzma/common/vli_encoder.c",
    "src/liblzma/common/vli_size.c",
    "src/liblzma/delta/delta_common.c",
    "src/liblzma/delta/delta_decoder.c",
    "src/liblzma/delta/delta_encoder.c",
    "src/liblzma/lz/lz_decoder.c",
    "src/liblzma/lz/lz_encoder.c",
    "src/liblzma/lz/lz_encoder_mf.c",
    "src/liblzma/lzma/fastpos_table.c",
    "src/liblzma/lzma/lzma2_decoder.c",
    "src/liblzma/lzma/lzma2_encoder.c",
    "src/liblzma/lzma/lzma_decoder.c",
    "src/liblzma/lzma/lzma_encoder.c",
    "src/liblzma/lzma/lzma_encoder_optimum_fast.c",
    "src/liblzma/lzma/lzma_encoder_optimum_normal.c",
    "src/liblzma/lzma/lzma_encoder_presets.c",
    "src/liblzma/rangecoder/price_table.c",
    "src/liblzma/simple/arm.c",
    "src/liblzma/simple/arm64.c",
    "src/liblzma/simple/armthumb.c",
    "src/liblzma/simple/ia64.c",
    "src/liblzma/simple/powerpc.c",
    "src/liblzma/simple/riscv.c",
    "src/liblzma/simple/simple_coder.c",
    "src/liblzma/simple/simple_decoder.c",
    "src/liblzma/simple/simple_encoder.c",
    "src/liblzma/simple/sparc.c",
    "src/liblzma/simple/x86.c",
};

const libdeflate_sources = &.{
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
};

const htslib_sources = &.{
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
    "simd.c",
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
};

const common_flags = &.{"-fPIC"};

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const bzip2 = b.option(bool, "bzip2", "Enable bzip2 support") orelse false;
    const lzma = b.option(bool, "lzma", "Enable liblzma support") orelse false;
    const libdeflate = b.option(bool, "libdeflate", "Enable libdeflate support") orelse false;
    const curl = b.option(bool, "curl", "Enable libcurl support") orelse false;
    const s3 = b.option(bool, "s3", "Enable S3 support") orelse false;
    const gcs = b.option(bool, "gcs", "Enable GCS support") orelse false;

    if ((s3 or gcs) and !curl) {
        std.debug.panic("S3 and GCS require curl", .{});
    }

    const compression = b.step("compression", "Build and install compression libraries");
    addCompression(b, compression, target, bzip2, lzma, libdeflate);

    const htslib = b.step("htslib", "Build and install HTSlib");
    addHtslib(b, htslib, target, bzip2, lzma, libdeflate, curl, s3, gcs);
}

fn addCompression(
    b: *std.Build,
    step: *std.Build.Step,
    target: std.Build.ResolvedTarget,
    bzip2: bool,
    lzma: bool,
    libdeflate: bool,
) void {
    const zlib = addLibrary(b, "z", target, "vendor/zlib", zlib_sources, &.{"."}, &.{
        "-fPIC",
        "-DHAVE_UNISTD_H=1",
    });
    installArtifact(b, step, zlib);
    installHeader(b, step, "vendor/zlib/zlib.h", "zlib.h");
    installHeader(b, step, "vendor/zlib/zconf.h", "zconf.h");

    if (bzip2) {
        const bz2 = addLibrary(b, "bz2", target, "vendor/bzip2", bzip2_sources, &.{"."}, common_flags);
        installArtifact(b, step, bz2);
        installHeader(b, step, "vendor/bzip2/bzlib.h", "bzlib.h");
    }

    if (lzma) {
        const lib = addLibrary(b, "lzma", target, "vendor/xz", lzma_sources, &.{
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
        }, &.{
            "-fPIC",
            "-DHAVE_CONFIG_H=1",
            "-std=c99",
        });
        installArtifact(b, step, lib);
        installHeader(b, step, "vendor/xz/src/liblzma/api/lzma.h", "lzma.h");
        const headers = b.addInstallDirectory(.{
            .source_dir = b.path("vendor/xz/src/liblzma/api/lzma"),
            .install_dir = .header,
            .install_subdir = "lzma",
            .include_extensions = &.{".h"},
        });
        step.dependOn(&headers.step);
    }

    if (libdeflate) {
        const lib = addLibrary(b, "deflate", target, "vendor/libdeflate", libdeflate_sources, &.{ ".", "lib" }, common_flags);
        installArtifact(b, step, lib);
        installHeader(b, step, "vendor/libdeflate/libdeflate.h", "libdeflate.h");
    }
}

fn addHtslib(
    b: *std.Build,
    step: *std.Build.Step,
    target: std.Build.ResolvedTarget,
    bzip2: bool,
    lzma: bool,
    libdeflate: bool,
    curl: bool,
    s3: bool,
    gcs: bool,
) void {
    const config = b.fmt(
        \\#define HAVE_DRAND48 1
        \\{s}{s}{s}{s}{s}{s}{s}{s}
    , .{
        enabled(bzip2, "#define HAVE_LIBBZ2 1\n"),
        enabled(lzma, "#define HAVE_LZMA_H 1\n"),
        enabled(libdeflate, "#define HAVE_LIBDEFLATE 1\n"),
        enabled(curl, "#define HAVE_LIBCURL 1\n"),
        enabled(s3, "#define ENABLE_S3 1\n"),
        enabled(gcs, "#define ENABLE_GCS 1\n"),
        enabled(curl, "#define HAVE_HMAC 1\n"),
        enabled(lzma, "#define HAVE_LIBLZMA 1\n"),
    });
    const generated = b.addWriteFiles();
    const config_h = generated.add("config.h", config);
    _ = generated.add(
        "version.h",
        "#define HTS_VERSION_TEXT \"1.24\"\n#define HTSCODECS_VERSION_TEXT \"1.6.7\"\n",
    );
    _ = generated.add(
        "config_vars.h",
        "#define HTS_CC \"zig cc 0.15.2\"\n#define HTS_CPPFLAGS \"\"\n#define HTS_CFLAGS \"-OReleaseSafe -fPIC -mcpu=baseline\"\n#define HTS_LDFLAGS \"\"\n#define HTS_LIBS \"vendored static libraries\"\n",
    );

    const module = b.createModule(.{
        .target = target,
        .optimize = .ReleaseSafe,
        // HTSlib's threaded CRAM encoder trips Zig's C sanitizer traps.
        .sanitize_c = .off,
        .link_libc = true,
        .pic = true,
    });
    module.addIncludePath(config_h.dirname());
    module.addIncludePath(b.path("vendor/htslib"));
    module.addIncludePath(.{ .cwd_relative = b.getInstallPath(.header, "") });
    module.addCSourceFiles(.{
        .root = b.path("vendor/htslib"),
        .files = htslib_sources,
        .flags = &.{ "-fPIC", "-Wno-deprecated-declarations" },
    });
    if (curl) addCSource(module, b, "vendor/htslib/hfile_libcurl.c");
    if (s3) addCSource(module, b, "vendor/htslib/hfile_s3.c");
    if (gcs) addCSource(module, b, "vendor/htslib/hfile_gcs.c");
    addCSource(module, b, "native/wrapper.c");

    const lib = b.addLibrary(.{
        .name = "hts",
        .linkage = .static,
        .root_module = module,
    });
    installArtifact(b, step, lib);
    const headers = b.addInstallDirectory(.{
        .source_dir = b.path("vendor/htslib/htslib"),
        .install_dir = .header,
        .install_subdir = "htslib",
        .include_extensions = &.{".h"},
    });
    step.dependOn(&headers.step);
}

fn addLibrary(
    b: *std.Build,
    name: []const u8,
    target: std.Build.ResolvedTarget,
    root: []const u8,
    sources: []const []const u8,
    includes: []const []const u8,
    flags: []const []const u8,
) *std.Build.Step.Compile {
    const module = b.createModule(.{
        .target = target,
        .optimize = .ReleaseSafe,
        .link_libc = true,
        .pic = true,
    });
    for (includes) |include| {
        module.addIncludePath(b.path(b.pathJoin(&.{ root, include })));
    }
    module.addCSourceFiles(.{
        .root = b.path(root),
        .files = sources,
        .flags = flags,
    });
    return b.addLibrary(.{
        .name = name,
        .linkage = .static,
        .root_module = module,
    });
}

fn addCSource(module: *std.Build.Module, b: *std.Build, path: []const u8) void {
    module.addCSourceFile(.{
        .file = b.path(path),
        .flags = &.{ "-fPIC", "-Wno-deprecated-declarations" },
    });
}

fn installArtifact(b: *std.Build, step: *std.Build.Step, artifact: *std.Build.Step.Compile) void {
    const install = b.addInstallArtifact(artifact, .{});
    step.dependOn(&install.step);
}

fn installHeader(b: *std.Build, step: *std.Build.Step, source: []const u8, destination: []const u8) void {
    const install = b.addInstallHeaderFile(b.path(source), destination);
    step.dependOn(&install.step);
}

fn enabled(value: bool, line: []const u8) []const u8 {
    return if (value) line else "";
}
