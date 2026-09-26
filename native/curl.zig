const std = @import("std");

pub fn build(b: *std.Build, step: *std.Build.Step, target: std.Build.ResolvedTarget) void {
    const module = b.createModule(.{ .target = target, .optimize = .ReleaseSafe, .link_libc = true, .pic = true });
    module.addIncludePath(b.path("native"));
    module.addIncludePath(b.path("vendor/curl/include"));
    module.addIncludePath(b.path("vendor/curl/lib"));
    module.addIncludePath(.{ .cwd_relative = b.getInstallPath(.header, "") });
    module.addCMacro("HAVE_CONFIG_H", "1");
    module.addCMacro("BUILDING_LIBCURL", "1");
    module.addCMacro("CURL_HIDDEN_SYMBOLS", "1");
    if (target.result.os.tag == .linux) module.addCMacro("_GNU_SOURCE", "1");
    if (target.result.abi.isGnu()) module.addCMacro("NANALOGUE_CURL_GNU", "1");
    if (target.result.os.tag == .macos) {
        if (b.sysroot orelse b.graph.env_map.get("SDKROOT")) |sdk| {
            module.addSystemIncludePath(.{ .cwd_relative = b.pathJoin(&.{ sdk, "usr/include" }) });
            module.addSystemFrameworkPath(.{ .cwd_relative = b.pathJoin(&.{ sdk, "System/Library/Frameworks" }) });
        }
    }
    module.addCSourceFiles(.{
        .root = b.path("vendor/curl/lib"),
        .files = @import("curl-sources.zig").sources,
        .flags = &.{ "-fPIC", "-pthread", "-fvisibility=hidden", "-Werror=implicit-function-declaration" },
    });
    const lib = b.addLibrary(.{ .name = "curl", .linkage = .static, .root_module = module });
    step.dependOn(&b.addInstallArtifact(lib, .{}).step);
    step.dependOn(&b.addInstallDirectory(.{
        .source_dir = b.path("vendor/curl/include/curl"),
        .install_dir = .header,
        .install_subdir = "curl",
        .include_extensions = &.{".h"},
    }).step);
}
