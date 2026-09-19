#!/usr/bin/env bash
# Explicit toolchain setup, never invoked by build.rs. Verified Zig 0.14.1.
set -euo pipefail
if command -v zig >/dev/null && [ "$(zig version)" = 0.14.1 ]; then
    exit 0
fi
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) platform=x86_64-linux; sha=24aeeec8af16c381934a6cd7d95c807a8cb2cf7df9fa40d359aa884195c4716c ;;
    Linux-aarch64) platform=aarch64-linux; sha=f7a654acc967864f7a050ddacfaa778c7504a0eca8d2b678839c21eea47c992b ;;
    Darwin-x86_64) platform=x86_64-macos; sha=b0f8bdfb9035783db58dd6c19d7dea89892acc3814421853e5752fe4573e5f43 ;;
    Darwin-arm64) platform=aarch64-macos; sha=39f3dc5e79c22088ce878edc821dedb4ca5a1cd9f5ef915e9b3cc3053e8faefa ;;
    *) echo "Unsupported Zig host" >&2; exit 1 ;;
esac
prefix="$HOME/.local/lib/zig"
mkdir -p "$prefix" "$HOME/.local/bin"
if [ ! -x "$prefix/zig-$platform-0.14.1/zig" ]; then
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    curl --fail --location --retry 3 "https://ziglang.org/download/0.14.1/zig-$platform-0.14.1.tar.xz" -o "$tmp/zig.tar.xz"
    if command -v sha256sum >/dev/null; then
        echo "$sha  $tmp/zig.tar.xz" | sha256sum -c -
    else
        echo "$sha  $tmp/zig.tar.xz" | shasum -a 256 -c -
    fi
    tar -xJf "$tmp/zig.tar.xz" -C "$prefix"
fi
ln -sf "$prefix/zig-$platform-0.14.1/zig" "$HOME/.local/bin/zig"
echo 'Zig installed. Add $HOME/.local/bin to PATH.'
