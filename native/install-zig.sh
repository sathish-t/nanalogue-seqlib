#!/usr/bin/env bash
# Explicit toolchain setup, never invoked by build.rs. Verified Zig 0.15.2.
set -euo pipefail
if command -v zig >/dev/null && [ "$(zig version)" = 0.15.2 ]; then
    exit 0
fi
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) platform=x86_64-linux; sha=02aa270f183da276e5b5920b1dac44a63f1a49e55050ebde3aecc9eb82f93239 ;;
    Linux-aarch64) platform=aarch64-linux; sha=958ed7d1e00d0ea76590d27666efbf7a932281b3d7ba0c6b01b0ff26498f667f ;;
    Darwin-x86_64) platform=x86_64-macos; sha=375b6909fc1495d16fc2c7db9538f707456bfc3373b14ee83fdd3e22b3d43f7f ;;
    Darwin-arm64) platform=aarch64-macos; sha=3cc2bab367e185cdfb27501c4b30b1b0653c28d9f73df8dc91488e66ece5fa6b ;;
    *) echo "Unsupported Zig host" >&2; exit 1 ;;
esac
prefix="$HOME/.local/lib/zig"
mkdir -p "$prefix" "$HOME/.local/bin"
if [ ! -x "$prefix/zig-$platform-0.15.2/zig" ]; then
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    curl --fail --location --retry 3 "https://ziglang.org/download/0.15.2/zig-$platform-0.15.2.tar.xz" -o "$tmp/zig.tar.xz"
    if command -v sha256sum >/dev/null; then
        echo "$sha  $tmp/zig.tar.xz" | sha256sum -c -
    else
        echo "$sha  $tmp/zig.tar.xz" | shasum -a 256 -c -
    fi
    tar -xJf "$tmp/zig.tar.xz" -C "$prefix"
fi
ln -sf "$prefix/zig-$platform-0.15.2/zig" "$HOME/.local/bin/zig"
echo 'Zig installed. Add $HOME/.local/bin to PATH.'
