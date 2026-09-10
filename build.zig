const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const htslib_script =
        \\if [ ! -f configure ]; then autoreconf -i; fi
        \\CC="$1 cc" ./configure --disable-bz2 --disable-lzma --disable-libcurl --disable-plugins --without-libdeflate
        \\make libhts.a
    ;

    const htslib = b.addSystemCommand(&.{
        "sh",
        "-c",
        htslib_script,
        "sh",
        b.graph.zig_exe,
    });
    htslib.setCwd(b.path("vendor/htslib"));

    const facade = b.addStaticLibrary(.{
        .name = "nanalogue_fai",
        .root_source_file = b.path("native/fai.zig"),
        .target = target,
        .optimize = optimize,
    });
    facade.addIncludePath(b.path("vendor/htslib"));
    facade.addIncludePath(b.path("vendor/htslib/htslib"));
    facade.linkLibC();
    facade.step.dependOn(&htslib.step);

    b.installArtifact(facade);
}
