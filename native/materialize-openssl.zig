//! Maintainer-only: materialize a fresh inspect-openssl oracle into golden inputs.
//! zig run native/materialize-openssl.zig -- target/openssl-oracle [--check]
const std = @import("std");

fn materialize(a: std.mem.Allocator, source: []const u8, dest: []const u8, check: bool) !void {
    const cwd = std.fs.cwd();
    if (check) {
        const expected = try cwd.readFileAlloc(a, source, 16 * 1024 * 1024);
        const actual = try cwd.readFileAlloc(a, dest, 16 * 1024 * 1024);
        if (!std.mem.eql(u8, expected, actual)) {
            std.debug.print("golden differs: {s}\n", .{dest});
            return error.GoldenDrift;
        }
    } else {
        try cwd.makePath(std.fs.path.dirname(dest).?);
        try cwd.copyFile(source, cwd, dest, .{});
    }
}

pub fn main() !void {
    var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena.deinit();
    const a = arena.allocator();
    const args = try std.process.argsAlloc(a);
    if (args.len < 2 or args.len > 3) return error.ExpectedOracleDirectory;
    const check = args.len == 3;
    if (check and !std.mem.eql(u8, args[2], "--check")) return error.ExpectedCheck;
    const cwd = std.fs.cwd();
    const comparison_path = try std.fs.path.join(a, &.{ args[1], "comparison.json" });
    const comparison = (try std.json.parseFromSlice(std.json.Value, a, try cwd.readFileAlloc(a, comparison_path, 16 * 1024 * 1024), .{})).value;
    if (!comparison.object.get("compile_groups_identical").?.bool or !comparison.object.get("disabled_options_identical").?.bool) return error.OracleDrift;
    const differences = comparison.object.get("source_differences_from_linux_x86_64").?.object;
    for (differences.values()) |libs| for (libs.object.values()) |lib| {
        for ([_][]const u8{ "added", "removed", "changed_object_settings" }) |key|
            if (lib.object.get(key).?.array.items.len != 0) return error.OracleDrift;
    };
    try materialize(a, comparison_path, "native/openssl/comparison.json", check);
    try materialize(a, try std.fs.path.join(a, &.{ args[1], "linux-x86_64/manifest.json" }), "native/openssl/manifest.json", check);
    for (comparison.object.get("common_generated_files").?.array.items) |path| {
        const dest = try std.fs.path.join(a, &.{ "native/openssl/common", path.string });
        try materialize(a, try std.fs.path.join(a, &.{ args[1], "linux-x86_64", path.string }), dest, check);
    }
    const varying = comparison.object.get("varying_generated_files").?.object;
    for (varying.keys(), varying.values()) |path, groups| {
        if (std.mem.eql(u8, path, "crypto/buildinf.h")) continue;
        for (groups.array.items) |group| {
            const config = group.array.items[0].string;
            const dest = try std.fs.path.join(a, &.{ "native/openssl/overlays", config, path });
            try materialize(a, try std.fs.path.join(a, &.{ args[1], config, path }), dest, check);
        }
    }
}
