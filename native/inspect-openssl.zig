//! Maintainer-only pinned Configure oracle; NEVER called by consumer builds.
//! Run at the repository root: zig run native/inspect-openssl.zig -- OUTPUT [--archives DIR]
//! Requires Perl/Make for the upstream oracle, but generates no object code.
const std = @import("std");
const Allocator = std.mem.Allocator;
const Value = std.json.Value;
const configs = [_][]const u8{
    "linux-x86_64",    "linux-aarch64",      "linux-armv4",       "linux-ppc64le",
    "linux64-riscv64", "darwin64-x86_64-cc", "darwin64-arm64-cc",
};
const options = [_][]const u8{
    "no-shared", "no-tests", "no-module", "no-dso", "no-engine", "no-asm", "-fPIC",
};
// Query the authoritative Configure data through Perl, not an interpreter for
// Make/Perl implemented in Zig. Keep object ownership and internal dependencies:
// following only sources[libcrypto] loses the 28 provider-common objects.
const extract =
    \\use configdata;
    \\use JSON::PP;
    \\my %result;
    \\for my $library (qw(libcrypto libssl)) {
    \\    my %objects;
    \\    my %seen;
    \\    my $walk;
    \\    $walk = sub {
    \\        my ($node, $owner, $parent) = @_;
    \\        if ($node eq $library || $node =~ /\.a$/) {
    \\            return if $seen{$node}++;
    \\            $owner = $node;
    \\            $result{compile_groups}{$owner} = {
    \\                defines => $unified_info{defines}{$owner} // [],
    \\                includes => $unified_info{includes}{$owner} // [],
    \\            };
    \\            $walk->($_, "", "") for grep { /\.a$/ } @{$unified_info{depends}{$node} // []};
    \\        }
    \\        if ($node =~ /\.c$/) {
    \\            die "source without object parent: $node" unless $parent =~ /\.o$/;
    \\            die "object has multiple sources: $parent" if exists $objects{$parent};
    \\            $objects{$parent} = {
    \\                source => $node, owner => $owner,
    \\                defines => $unified_info{defines}{$parent} // [],
    \\                includes => $unified_info{includes}{$parent} // [],
    \\            };
    \\        } elsif (exists $unified_info{sources}{$node}) {
    \\            $walk->($_, $owner, $node) for @{$unified_info{sources}{$node}};
    \\        } else {
    \\            die "unclassified source node: $node";
    \\        }
    \\    };
    \\    $walk->($library, "", "");
    \\    $result{libraries}{$library} = \%objects;
    \\}
    \\$result{generated} = [sort grep { /\.(c|h|inc)$/ } keys %{$unified_info{generate}}];
    \\$result{target} = {map {$_ => $target{$_}} qw(bn_ops lib_cppflags thread_scheme ex_libs)};
    \\$result{disabled} = [sort keys %disabled];
    \\print JSON::PP->new->canonical->encode(\%result);
;

fn fail(comptime format: []const u8, args: anytype) noreturn {
    std.debug.print("error: " ++ format ++ "\n", args);
    std.process.exit(1);
}

fn say(a: Allocator, comptime format: []const u8, args: anytype) !void {
    try std.fs.File.stdout().writeAll(try std.fmt.allocPrint(a, format ++ "\n", args));
}

fn read(a: Allocator, path: []const u8) []const u8 {
    return std.fs.cwd().readFileAlloc(a, path, 32 * 1024 * 1024) catch |err|
        fail("read {s}: {s}", .{ path, @errorName(err) });
}

fn object(a: Allocator) Value {
    return .{ .object = std.json.ObjectMap.init(a) };
}

fn array(a: Allocator) Value {
    return .{ .array = std.json.Array.init(a) };
}

fn field(value: Value, name: []const u8) Value {
    if (value != .object) fail("oracle JSON: expected object containing {s}", .{name});
    return value.object.get(name) orelse fail("oracle JSON: missing {s}", .{name});
}

fn less(_: void, left: []const u8, right: []const u8) bool {
    return std.mem.lessThan(u8, left, right);
}

fn keys(a: Allocator, value: Value) ![][]const u8 {
    const result = try a.dupe([]const u8, value.object.keys());
    std.mem.sort([]const u8, result, {}, less);
    return result;
}

// Preserve array ordering (include path precedence), but sort every object key
// like the former sort_keys=True JSON. Unicode escaping also matches that tool.
fn canonical(a: Allocator, value: Value) Allocator.Error!Value {
    switch (value) {
        .object => {
            var result = object(a);
            for (try keys(a, value)) |key| try result.object.put(key, try canonical(a, field(value, key)));
            return result;
        },
        .array => |items| {
            var result = array(a);
            for (items.items) |item| try result.array.append(try canonical(a, item));
            return result;
        },
        else => return value,
    }
}

fn equal(left: Value, right: Value) bool {
    if (std.meta.activeTag(left) != std.meta.activeTag(right)) return false;
    return switch (left) {
        .null => true,
        .bool => |v| v == right.bool,
        .integer => |v| v == right.integer,
        .float => |v| v == right.float,
        .string => |v| std.mem.eql(u8, v, right.string),
        .number_string => |v| std.mem.eql(u8, v, right.number_string),
        .array => |v| blk: {
            if (v.items.len != right.array.items.len) break :blk false;
            for (v.items, right.array.items) |l, r| if (!equal(l, r)) break :blk false;
            break :blk true;
        },
        .object => |v| blk: {
            if (v.count() != right.object.count()) break :blk false;
            for (v.keys(), v.values()) |key, item| {
                const other = right.object.get(key) orelse break :blk false;
                if (!equal(item, other)) break :blk false;
            }
            break :blk true;
        },
    };
}

fn writeJson(a: Allocator, path: []const u8, value: Value) !void {
    const text = try std.json.Stringify.valueAlloc(a, try canonical(a, value), .{ .whitespace = .indent_2, .escape_unicode = true });
    std.fs.cwd().writeFile(.{ .sub_path = path, .data = try std.mem.concat(a, u8, &.{ text, "\n" }) }) catch |err|
        fail("write {s}: {s}", .{ path, @errorName(err) });
}

fn command(a: Allocator, argv: []const []const u8, cwd: []const u8, env: *const std.process.EnvMap, log: ?std.fs.File) ![]const u8 {
    const result = std.process.Child.run(.{
        .allocator = a,
        .argv = argv,
        .cwd = cwd,
        .env_map = env,
        .max_output_bytes = 32 * 1024 * 1024,
    }) catch |err| fail("execute {s} in {s}: {s}; check PATH (Perl/Make) and ZIG for archive checks", .{ argv[0], cwd, @errorName(err) });
    if (log) |file| {
        try file.writeAll(result.stdout);
        try file.writeAll(result.stderr);
    } else if (result.stderr.len != 0) {
        try std.fs.File.stderr().writeAll(result.stderr);
    }
    if (result.term != .Exited or result.term.Exited != 0) {
        if (log != null) fail("{s} failed in {s} ({any}); see {s}/oracle.log", .{ argv[0], cwd, result.term, cwd });
        fail("{s} failed in {s} ({any}); see command stderr above", .{ argv[0], cwd, result.term });
    }
    return result.stdout;
}

fn sanitizedEnv(a: Allocator) !std.process.EnvMap {
    var env = try std.process.getEnvMap(a);
    for ([_][]const u8{ "CFLAGS", "CXXFLAGS", "CPPFLAGS", "LDFLAGS", "LDLIBS", "CROSS_COMPILE", "OPENSSL_LOCAL_CONFIG_DIR", "CONFIGURE_ARGS" }) |name| env.remove(name);
    try env.put("SOURCE_DATE_EPOCH", "0");
    try env.put("CC", "zig cc");
    try env.put("AR", "zig ar");
    try env.put("RANLIB", "zig ar s");
    return env;
}

fn snapshot(a: Allocator, source: []const u8, output: []const u8, config: []const u8, env: *const std.process.EnvMap) !Value {
    const build = try std.fs.path.join(a, &.{ output, config });
    try std.fs.cwd().makeDir(build);
    const log = try std.fs.cwd().createFile(try std.fs.path.join(a, &.{ build, "oracle.log" }), .{ .exclusive = true });
    defer log.close();
    var configure: std.ArrayList([]const u8) = .empty;
    try configure.appendSlice(a, &.{ "perl", try std.fs.path.join(a, &.{ source, "Configure" }), config, "--prefix=/native", "--libdir=lib", "--openssldir=/etc/ssl" });
    try configure.appendSlice(a, &options);
    _ = try command(a, configure.items, build, env, log);
    const raw = try command(a, &.{ "perl", "-I.", "-e", extract }, build, env, null);
    const relative = try std.fs.path.relative(a, build, source);
    const normalized = try std.mem.replaceOwned(u8, a, raw, try std.mem.concat(a, u8, &.{ relative, "/" }), "@source/");
    const normalized_root = try std.mem.replaceOwned(u8, a, normalized, try std.fmt.allocPrint(a, "\"{s}\"", .{relative}), "\"@source\"");
    var data = (std.json.parseFromSlice(Value, a, normalized_root, .{ .allocate = .alloc_always }) catch |err|
        fail("invalid Configure JSON for {s}: {s}", .{ config, @errorName(err) })).value;
    var wanted = std.StringHashMap(void).init(a);
    for (field(data, "generated").array.items) |item| {
        const path = item.string;
        if (std.mem.startsWith(u8, path, "include/") or std.mem.startsWith(u8, path, "providers/") or std.mem.startsWith(u8, path, "crypto/"))
            try wanted.put(path, {});
    }
    var make: std.ArrayList([]const u8) = .empty;
    try make.appendSlice(a, &.{ "make", "-j2" });
    var it = wanted.keyIterator();
    while (it.next()) |path| try make.append(a, path.*);
    std.mem.sort([]const u8, make.items[2..], {}, less);
    _ = try command(a, make.items, build, env, log);
    // Configure itself emits this header, not unified_info{generate}.
    try wanted.put("include/openssl/configuration.h", {});
    var hashes = object(a);
    it = wanted.keyIterator();
    while (it.next()) |path| {
        var hash: [32]u8 = undefined;
        std.crypto.hash.sha2.Sha256.hash(read(a, try std.fs.path.join(a, &.{ build, path.* })), &hash, .{});
        try hashes.object.put(path.*, .{ .string = try a.dupe(u8, &std.fmt.bytesToHex(hash, .lower)) });
    }
    try data.object.put("generated", hashes);
    try writeJson(a, try std.fs.path.join(a, &.{ build, "manifest.json" }), data);
    const libs = field(data, "libraries");
    try say(a, "{s}: libcrypto={d} C objects, libssl={d} C objects, {d} generated files", .{ config, field(libs, "libcrypto").object.count(), field(libs, "libssl").object.count(), hashes.object.count() });
    return data;
}

fn archiveMembersMatch(a: Allocator, text: []const u8, objects: Value) !bool {
    var actual: std.ArrayList([]const u8) = .empty;
    var lines = std.mem.tokenizeAny(u8, text, "\r\n");
    while (lines.next()) |line| try actual.append(a, line);
    var expected: std.ArrayList([]const u8) = .empty;
    for (objects.object.keys()) |path| try expected.append(a, std.fs.path.basename(path));
    std.mem.sort([]const u8, actual.items, {}, less);
    std.mem.sort([]const u8, expected.items, {}, less);
    if (actual.items.len != expected.items.len) return false;
    for (actual.items, expected.items) |l, r| if (!std.mem.eql(u8, l, r)) return false;
    return true;
}

fn compare(a: Allocator, snapshots: Value, version: []const u8) !Value {
    const reference = field(snapshots, configs[0]);
    var paths = object(a);
    for (snapshots.object.values()) |data| {
        for (field(data, "generated").object.keys()) |path| try paths.object.put(path, .null);
    }
    var common = array(a);
    var varying = object(a);
    for (try keys(a, paths)) |path| {
        var groups = object(a);
        for (configs) |config| {
            const hashes = field(field(snapshots, config), "generated");
            const hash = if (hashes.object.get(path)) |v| v.string else "absent";
            const group = try groups.object.getOrPut(hash);
            if (!group.found_existing) group.value_ptr.* = array(a);
            try group.value_ptr.array.append(.{ .string = config });
        }
        if (groups.object.count() == 1) {
            try common.array.append(.{ .string = path });
        } else {
            var lists = array(a);
            try lists.array.appendSlice(groups.object.values());
            try varying.object.put(path, lists);
        }
    }
    var target_settings = object(a);
    var differences = object(a);
    var disabled_equal = true;
    var groups_equal = true;
    for (configs) |config| {
        const data = field(snapshots, config);
        try target_settings.object.put(config, field(data, "target"));
        disabled_equal = disabled_equal and equal(field(data, "disabled"), field(reference, "disabled"));
        groups_equal = groups_equal and equal(field(data, "compile_groups"), field(reference, "compile_groups"));
        var libraries = object(a);
        const libs = field(data, "libraries");
        for (libs.object.keys(), libs.object.values()) |lib, objects| {
            const base = field(field(reference, "libraries"), lib);
            var added = array(a);
            var removed = array(a);
            var changed = array(a);
            for (try keys(a, objects)) |path| {
                if (base.object.get(path)) |original| {
                    if (!equal(original, field(objects, path))) try changed.array.append(.{ .string = path });
                } else try added.array.append(.{ .string = path });
            }
            for (try keys(a, base)) |path| {
                if (!objects.object.contains(path)) try removed.array.append(.{ .string = path });
            }
            var diff = object(a);
            try diff.object.put("count", .{ .integer = @intCast(objects.object.count()) });
            try diff.object.put("added", added);
            try diff.object.put("removed", removed);
            try diff.object.put("changed_object_settings", changed);
            try libraries.object.put(lib, diff);
        }
        try differences.object.put(config, libraries);
    }
    var result = object(a);
    var option_list = array(a);
    for (options) |option| try option_list.array.append(.{ .string = option });
    try result.object.put("openssl_version", .{ .string = version });
    try result.object.put("options", option_list);
    try result.object.put("common_generated_files", common);
    try result.object.put("varying_generated_files", varying);
    try result.object.put("source_differences_from_linux_x86_64", differences);
    try result.object.put("target_settings", target_settings);
    try result.object.put("disabled_options_identical", .{ .bool = disabled_equal });
    try result.object.put("compile_groups_identical", .{ .bool = groups_equal });
    return result;
}

pub fn main() void {
    var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena.deinit();
    run(arena.allocator()) catch |err| fail("{s}", .{@errorName(err)});
}

fn run(a: Allocator) !void {
    const args = try std.process.argsAlloc(a);
    var output_arg: ?[]const u8 = null;
    var archives: ?[]const u8 = null;
    var i: usize = 1;
    while (i < args.len) : (i += 1) {
        const arg = args[i];
        if (std.mem.eql(u8, arg, "--help")) {
            try say(a, "Usage: zig run native/inspect-openssl.zig -- OUTPUT [--archives DIR]\nRun from the repository root. OUTPUT must not exist; its parents are created.\nRequires Perl and Make. --archives compares x86_64 Linux libcrypto.a/libssl.a membership.", .{});
            return;
        } else if (std.mem.eql(u8, arg, "--archives")) {
            i += 1;
            if (i == args.len or archives != null) fail("--archives requires one directory; use --help for usage", .{});
            archives = args[i];
        } else if (std.mem.startsWith(u8, arg, "-") or output_arg != null) {
            fail("unexpected argument {s}; use --help for usage", .{arg});
        } else output_arg = arg;
    }
    const root = try std.fs.cwd().realpathAlloc(a, ".");
    const source = try std.fs.path.join(a, &.{ root, "vendor/openssl" });
    const version = read(a, try std.fs.path.join(a, &.{ source, "VERSION.dat" }));
    const resolved = try std.fs.path.resolve(a, &.{ root, output_arg orelse fail("missing output directory; use --help for usage", .{}) });
    const parent = std.fs.path.dirname(resolved) orelse fail("invalid output directory: {s}", .{resolved});
    try std.fs.cwd().makePath(parent);
    const output = try std.fs.path.join(a, &.{ try std.fs.cwd().realpathAlloc(a, parent), std.fs.path.basename(resolved) });
    std.fs.cwd().makeDir(output) catch |err|
        fail("create {s}: {s}; choose a new disposable output directory", .{ output, @errorName(err) });
    var env = try sanitizedEnv(a);
    defer env.deinit();
    var snapshots = object(a);
    for (configs) |config| try snapshots.object.put(config, try snapshot(a, source, output, config, &env));
    if (archives) |directory| {
        const libs = field(field(snapshots, configs[0]), "libraries");
        for (libs.object.keys(), libs.object.values()) |lib, objects| {
            const archive = try std.fs.path.resolve(a, &.{ root, directory, try std.fmt.allocPrint(a, "{s}.a", .{lib}) });
            const actual = try command(a, &.{ env.get("ZIG") orelse "zig", "ar", "t", archive }, root, &env, null);
            if (!try archiveMembersMatch(a, actual, objects))
                fail("{s} archive membership differs: expected {d} objects; compare zig ar t {s} with {s}/{s}/manifest.json", .{ lib, objects.object.count(), archive, output, configs[0] });
            try say(a, "{s}: all {d} archive members match", .{ lib, objects.object.count() });
        }
    }
    const comparison = try compare(a, snapshots, version);
    const path = try std.fs.path.join(a, &.{ output, "comparison.json" });
    try writeJson(a, path, comparison);
    try say(a, "{d} shared generated files; {d} varying files. See {s}", .{ field(comparison, "common_generated_files").array.items.len, field(comparison, "varying_generated_files").object.count(), path });
}

test "canonical JSON sorts objects but preserves include precedence" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const a = arena.allocator();
    const original = (try std.json.parseFromSlice(Value, a, "{\"z\":{\"b\":[],\"a\":{}},\"a\":[\"second\",\"first\"]}", .{})).value;
    const expected = "{\"a\":[\"second\",\"first\"],\"z\":{\"a\":{},\"b\":[]}}";
    const sorted = try canonical(a, original);
    try std.testing.expectEqualStrings(expected, try std.json.Stringify.valueAlloc(a, sorted, .{}));
    try std.testing.expect(equal(original, sorted));
}

test "comparison detects missing hashes and added removed and changed objects" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const a = arena.allocator();
    const baseline = (try std.json.parseFromSlice(Value, a,
        \\{"generated":{"shared.h":"same","vary.h":"a","missing.h":"m"},
        \\ "disabled":["no-asm"],"compile_groups":{"includes":["first","second"]},
        \\ "target":{},"libraries":{"libcrypto":{
        \\ "keep.o":{"includes":["first","second"]},"remove.o":{}}}}
    , .{})).value;
    const modified = (try std.json.parseFromSlice(Value, a,
        \\{"generated":{"shared.h":"same","vary.h":"b"},
        \\ "disabled":["no-asm","no-engine"],"compile_groups":{"includes":["second","first"]},
        \\ "target":{},"libraries":{"libcrypto":{
        \\ "keep.o":{"includes":["second","first"]},"add.o":{}}}}
    , .{})).value;
    var snapshots = object(a);
    for (configs, 0..) |config, index| try snapshots.object.put(config, if (index == 1) modified else baseline);
    const result = try compare(a, snapshots, "test-version");
    const common = field(result, "common_generated_files").array.items;
    try std.testing.expectEqual(@as(usize, 1), common.len);
    try std.testing.expectEqualStrings("shared.h", common[0].string);
    const varying = field(result, "varying_generated_files");
    try std.testing.expectEqual(@as(usize, 2), varying.object.count());
    for ([_][]const u8{ "vary.h", "missing.h" }) |path| {
        const groups = field(varying, path).array.items;
        try std.testing.expectEqual(@as(usize, 2), groups.len);
        try std.testing.expectEqual(@as(usize, 6), groups[0].array.items.len);
        try std.testing.expectEqual(@as(usize, 1), groups[1].array.items.len);
        try std.testing.expectEqualStrings(configs[1], groups[1].array.items[0].string);
    }
    const diff = field(field(field(result, "source_differences_from_linux_x86_64"), configs[1]), "libcrypto");
    try std.testing.expectEqual(@as(i64, 2), field(diff, "count").integer);
    inline for (.{ .{ "added", "add.o" }, .{ "removed", "remove.o" }, .{ "changed_object_settings", "keep.o" } }) |pair| {
        const entries = field(diff, pair[0]).array.items;
        try std.testing.expectEqual(@as(usize, 1), entries.len);
        try std.testing.expectEqualStrings(pair[1], entries[0].string);
    }
    try std.testing.expect(!field(result, "disabled_options_identical").bool);
    try std.testing.expect(!field(result, "compile_groups_identical").bool);
}

test "archive comparison rejects duplicate missing and extra members" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const a = arena.allocator();
    const objects = (try std.json.parseFromSlice(Value, a, "{\"crypto/a.o\":{},\"providers/b.o\":{}}", .{})).value;
    try std.testing.expect(try archiveMembersMatch(a, "b.o\na.o\n", objects));
    for ([_][]const u8{ "a.o\n", "a.o\na.o\n", "a.o\nb.o\nc.o\n", "a.o\nc.o\n" }) |bad| {
        try std.testing.expect(!try archiveMembersMatch(a, bad, objects));
    }
}
