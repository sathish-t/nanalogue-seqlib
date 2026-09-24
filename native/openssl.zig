const std = @import("std");
const Value = std.json.Value;

fn get(v: Value, key: []const u8) Value {
    return v.object.get(key) orelse @panic("incomplete OpenSSL manifest");
}

pub fn build(b: *std.Build, step: *std.Build.Step, target: std.Build.ResolvedTarget) void {
    const config = switch (target.result.os.tag) {
        .macos => switch (target.result.cpu.arch) {
            .x86_64 => "darwin64-x86_64-cc",
            .aarch64 => "darwin64-arm64-cc",
            else => @panic("unsupported OpenSSL macOS architecture"),
        },
        .linux => switch (target.result.cpu.arch) {
            .x86_64 => "linux-x86_64",
            .aarch64 => "linux-aarch64",
            .arm => "linux-armv4",
            .powerpc64le => "linux-ppc64le",
            .riscv64 => "linux64-riscv64",
            else => @panic("unsupported OpenSSL Linux architecture"),
        },
        else => @panic("unsupported OpenSSL OS"),
    };
    const manifest = (std.json.parseFromSlice(Value, b.allocator, @embedFile("openssl/manifest.json"), .{}) catch @panic("invalid manifest")).value;
    const comparison = (std.json.parseFromSlice(Value, b.allocator, @embedFile("openssl/comparison.json"), .{}) catch @panic("invalid comparison")).value;
    const generated = b.addWriteFiles();
    _ = generated.add("crypto/buildinf.h", b.fmt(
        "#define PLATFORM \"platform: {s}\"\n#define DATE \"built on: reproducible build\"\nstatic const char compiler_flags[] = \"compiler: Zig 0.15.2 ReleaseSafe -fPIC no-asm\";\n",
        .{config},
    ));
    const varying = get(comparison, "varying_generated_files").object;
    for (varying.keys(), varying.values()) |path, groups| {
        if (std.mem.eql(u8, path, "crypto/buildinf.h")) continue;
        for (groups.array.items) |group| {
            for (group.array.items) |member| {
                if (std.mem.eql(u8, member.string, config)) {
                    _ = generated.addCopyFile(b.path(b.fmt("native/openssl/overlays/{s}/{s}", .{ group.array.items[0].string, path })), path);
                }
            }
        }
    }
    const test_module = b.createModule(.{ .target = target, .optimize = .ReleaseSafe, .link_libc = true });
    test_module.addIncludePath(generated.getDirectory().path(b, "include"));
    test_module.addIncludePath(b.path("native/openssl/common/include"));
    test_module.addIncludePath(b.path("vendor/openssl/include"));
    test_module.addCSourceFile(.{ .file = b.path("native/openssl-stack-test.c"), .flags = &.{"-pthread"} });
    test_module.linkSystemLibrary("pthread", .{});
    const libraries = get(manifest, "libraries").object;
    for (libraries.keys(), libraries.values()) |name, objects| {
        const module = b.createModule(.{ .target = target, .optimize = .ReleaseSafe, .link_libc = true, .pic = true });
        if (target.result.os.tag == .macos) {
            if (b.sysroot orelse b.graph.env_map.get("SDKROOT")) |sdk| {
                module.addSystemIncludePath(.{ .cwd_relative = b.pathJoin(&.{ sdk, "usr/include" }) });
                module.addSystemFrameworkPath(.{ .cwd_relative = b.pathJoin(&.{ sdk, "System/Library/Frameworks" }) });
            }
        }
        module.addIncludePath(generated.getDirectory());
        module.addIncludePath(generated.getDirectory().path(b, "include"));
        module.addIncludePath(generated.getDirectory().path(b, "crypto"));
        const target_flags = get(get(get(comparison, "target_settings"), config), "lib_cppflags").string;
        for (objects.object.values()) |object| {
            var flags: std.ArrayList([]const u8) = .empty;
            flags.appendSlice(b.allocator, &.{ "-fPIC", "-pthread", "-DOPENSSL_PIC", "-DOPENSSL_BUILDING_OPENSSL", "-DNDEBUG", "-DOPENSSLDIR=\"/etc/ssl\"", "-DENGINESDIR=\"/native/lib/engines-3\"", "-DMODULESDIR=\"/native/lib/ossl-modules\"" }) catch @panic("OOM");
            var tokens = std.mem.tokenizeScalar(u8, target_flags, ' ');
            while (tokens.next()) |flag| flags.append(b.allocator, flag) catch @panic("OOM");
            const group = get(get(manifest, "compile_groups"), get(object, "owner").string);
            // Upstream prepends object-specific settings to the library group.
            for ([_]Value{ object, group }) |settings| {
                for (get(settings, "defines").array.items) |define| flags.append(b.allocator, b.fmt("-D{s}", .{define.string})) catch @panic("OOM");
                for (get(settings, "includes").array.items) |include| {
                    const p = include.string;
                    const full = if (std.mem.startsWith(u8, p, "@source")) b.fmt("vendor/openssl{s}", .{p[7..]}) else b.fmt("native/openssl/common/{s}", .{p});
                    flags.append(b.allocator, b.fmt("-I{s}", .{b.pathFromRoot(full)})) catch @panic("OOM");
                }
            }
            const source = get(object, "source").string;
            const path = if (std.mem.startsWith(u8, source, "@source/")) b.fmt("vendor/openssl/{s}", .{source[8..]}) else b.fmt("native/openssl/common/{s}", .{source});
            module.addCSourceFile(.{ .file = b.path(path), .flags = flags.items });
        }
        const library = b.addLibrary(.{ .name = name[3..], .linkage = .static, .root_module = module });
        step.dependOn(&b.addInstallArtifact(library, .{}).step);
        test_module.linkLibrary(library);
    }
    const exe = b.addExecutable(.{ .name = "openssl-stack-test", .root_module = test_module });
    b.step("test-openssl", "Run ReleaseSafe typed-stack and provider regressions").dependOn(&b.addRunArtifact(exe).step);
    b.step("openssl-test-build", "Link the OpenSSL regression executable without running it").dependOn(&b.addInstallArtifact(exe, .{}).step);
    for ([_]std.Build.LazyPath{ b.path("vendor/openssl/include/openssl"), b.path("native/openssl/common/include/openssl"), generated.getDirectory().path(b, "include/openssl") }) |dir| {
        step.dependOn(&b.addInstallDirectory(.{ .source_dir = dir, .install_dir = .header, .install_subdir = "openssl", .include_extensions = &.{".h"} }).step);
    }
}
