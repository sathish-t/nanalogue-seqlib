#!/usr/bin/env bash
# Install a checksum-verified minisign when the host has no package.
set -euo pipefail

if command -v minisign >/dev/null; then
    exit 0
fi

version=0.12
archive_name="minisign-$version-linux.tar.gz"
archive_sha256=9a599b48ba6eb7b1e80f12f36b94ceca7c00b7a5173c95c3efc88d9822957e73
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) platform=x86_64 ;;
    Linux-aarch64) platform=aarch64 ;;
    *) echo "Unsupported minisign host" >&2; exit 1 ;;
esac

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
archive="$tmp/$archive_name"
curl --proto '=https' --tlsv1.2 --connect-timeout 15 --max-time 60 \
    --retry 3 --fail --silent --show-error --location \
    --output "$archive" \
    "https://github.com/jedisct1/minisign/releases/download/$version/$archive_name"
printf '%s  %s\n' "$archive_sha256" "$archive" \
    | sha256sum --check --strict -
tar -xzf "$archive" -C "$tmp" \
    "minisign-linux/$platform/minisign"
mkdir -p "$HOME/.local/bin"
install -m 0755 "$tmp/minisign-linux/$platform/minisign" \
    "$HOME/.local/bin/minisign"
"$HOME/.local/bin/minisign" -v
