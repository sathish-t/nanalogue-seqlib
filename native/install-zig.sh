#!/usr/bin/env bash
# Explicit toolchain setup, never invoked by build.rs. Verified Zig 0.15.2.
set -euo pipefail

version=0.15.2
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) platform=x86_64-linux; sha=02aa270f183da276e5b5920b1dac44a63f1a49e55050ebde3aecc9eb82f93239 ;;
    Linux-aarch64) platform=aarch64-linux; sha=958ed7d1e00d0ea76590d27666efbf7a932281b3d7ba0c6b01b0ff26498f667f ;;
    Darwin-x86_64) platform=x86_64-macos; sha=375b6909fc1495d16fc2c7db9538f707456bfc3373b14ee83fdd3e22b3d43f7f ;;
    Darwin-arm64) platform=aarch64-macos; sha=3cc2bab367e185cdfb27501c4b30b1b0653c28d9f73df8dc91488e66ece5fa6b ;;
    *) echo "Unsupported Zig host" >&2; exit 1 ;;
esac

archive_name="zig-$platform-$version.tar.xz"
zsf_minisign_key="RWSGOq2NVecA2UPNdBUZykf1CCb147pkmdtYxgb3Ti+JO/wCYvhbAb/U"
prefix="$HOME/.local/lib/zig"
install_dir="$prefix/zig-$platform-$version"
checksum_marker="$install_dir/.archive.sha256"

if [ -x "$install_dir/zig" ] \
    && [ "$(cat "$checksum_marker" 2>/dev/null)" = "$sha" ] \
    && [ "$("$install_dir/zig" version)" = "$version" ]; then
    mkdir -p "$HOME/.local/bin"
    ln -sf "$install_dir/zig" "$HOME/.local/bin/zig"
    exit 0
fi

if ! command -v minisign >/dev/null; then
    echo "minisign is required to authenticate Zig downloads" >&2
    exit 1
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
archive="$tmp/$archive_name"
signature="$archive.minisig"
mirror_list="$tmp/community-mirrors.txt"
staging="$tmp/extracted"

curl --proto '=https' --tlsv1.2 --connect-timeout 15 --max-time 30 \
    --retry 3 --fail --silent --show-error --location \
    --output "$mirror_list" \
    https://ziglang.org/download/community-mirrors.txt

# Shuffle without GNU-only `shuf`, which is absent on macOS.
mirrors=()
while IFS= read -r mirror; do
    mirrors+=("$mirror")
done < <(awk 'BEGIN { srand() } NF && !seen[$0]++ {
    print rand() "\t" $0
}' "$mirror_list" | sort -n | cut -f2-)
if [ "${#mirrors[@]}" -eq 0 ]; then
    echo "No Zig community mirrors were returned" >&2
    exit 1
fi

verified=false
attempts=0
for mirror in "${mirrors[@]}"; do
    attempts=$((attempts + 1))
    if [ "$attempts" -gt 10 ]; then
        break
    fi
    if [[ "$mirror" != https://* || "$mirror" == *[[:space:]]* ]]; then
        echo "Invalid Zig community mirror URL: $mirror" >&2
        exit 1
    fi

    rm -f "$archive" "$signature"
    archive_url="${mirror%/}/$archive_name"
    if ! curl --proto '=https' --tlsv1.2 --connect-timeout 15 \
        --max-time 180 --fail --silent --show-error --location \
        --output "$archive" \
        "$archive_url?source=github-sathish-t-nanalogue-seqlib"; then
        continue
    fi
    if ! curl --proto '=https' --tlsv1.2 --connect-timeout 15 \
        --max-time 30 --fail --silent --show-error --location \
        --output "$signature" \
        "$archive_url.minisig?source=github-sathish-t-nanalogue-seqlib"; then
        continue
    fi
    if command -v sha256sum >/dev/null; then
        printf '%s  %s\n' "$sha" "$archive" | sha256sum --check --strict - \
            || continue
    else
        printf '%s  %s\n' "$sha" "$archive" | shasum -a 256 --check - \
            || continue
    fi
    minisign -Vm "$archive" -x "$signature" -P "$zsf_minisign_key" \
        || continue

    trusted_comment=$(sed -n '3s/^trusted comment: //p' "$signature")
    trusted_filename=""
    IFS=$'\t' read -ra comment_fields <<<"$trusted_comment"
    for field in "${comment_fields[@]:0:10}"; do
        if [[ "$field" == file:* ]]; then
            trusted_filename="${field#file:}"
        fi
    done
    if [ "$trusted_filename" != "$archive_name" ]; then
        continue
    fi

    verified=true
    break
done

if [ "$verified" != true ]; then
    echo "Could not verify Zig from 10 community mirrors" >&2
    exit 1
fi

mkdir "$staging"
tar -xJf "$archive" -C "$staging"
staged_install="$staging/zig-$platform-$version"
if [ ! -x "$staged_install/zig" ] \
    || [ "$("$staged_install/zig" version)" != "$version" ]; then
    echo "Verified Zig archive has unexpected contents" >&2
    exit 1
fi
printf '%s\n' "$sha" >"$staged_install/.archive.sha256"
mkdir -p "$prefix" "$HOME/.local/bin"
rm -rf "$install_dir"
mv "$staged_install" "$install_dir"
ln -sf "$install_dir/zig" "$HOME/.local/bin/zig"
echo 'Zig installed. Add $HOME/.local/bin to PATH.'
