#!/usr/bin/env bash
# Maintainer-only CMake oracle. NEVER called by Cargo or build.zig.
# Usage: bash native/inspect-curl.sh PREFIX ZIG_TARGET CPU NEW_OUTPUT
# PREFIX must contain direct Zig OpenSSL/zlib for the same target.
set -euo pipefail
unset CFLAGS CPPFLAGS CXXFLAGS LDFLAGS
prefix=$(realpath "$1")
target=$2
cpu=$3
out=$4
test ! -e "$out"
mkdir -p "$out"
out=$(realpath "$out")
zig=$(command -v "${ZIG:-zig}")
printf '#!/bin/bash\nexec %q cc -target %q -mcpu=%q "$@"\n' "$zig" "$target" "$cpu" > "$out/cc"
printf '#!/bin/bash\nexec %q ar "$@"\n' "$zig" > "$out/ar"
printf '#!/bin/bash\nexec %q ar s "$@"\n' "$zig" > "$out/ranlib"
chmod +x "$out/cc" "$out/ar" "$out/ranlib"
system=Linux
extra=()
if [[ "$target" == *macos* ]]; then
    system=Darwin
    extra=(-DUSE_APPLE_SECTRUST=ON "-DCMAKE_OSX_SYSROOT=${SDKROOT:?Apple SDK required}")
fi
args=()
for protocol in DICT DOH FILE GOPHER IMAP IPFS MQTT POP3 RTSP SMTP TELNET TFTP WEBSOCKETS LDAP LDAPS; do
    args+=("-DCURL_DISABLE_$protocol=ON")
done
cmake -S vendor/curl -B "$out/build" \
    -DCMAKE_BUILD_TYPE=Release -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    "-DCMAKE_C_COMPILER=$out/cc" "-DCMAKE_AR=$out/ar" "-DCMAKE_RANLIB=$out/ranlib" \
    "-DCMAKE_SYSTEM_NAME=$system" "-DCMAKE_SYSTEM_PROCESSOR=${target%%-*}" \
    -DBUILD_SHARED_LIBS=OFF -DBUILD_CURL_EXE=OFF -DBUILD_TESTING=OFF \
    -DBUILD_EXAMPLES=OFF -DBUILD_LIBCURL_DOCS=OFF -DBUILD_MISC_DOCS=OFF -DENABLE_CURL_MANUAL=OFF \
    -DCURL_USE_PKGCONFIG=OFF -DPKG_CONFIG_EXECUTABLE=/bin/false -DCURL_USE_CMAKECONFIG=OFF \
    -DCURL_CA_BUNDLE=none -DCURL_CA_PATH=none -DCURL_CA_FALLBACK=ON \
    -DCURL_USE_OPENSSL=ON "-DOPENSSL_ROOT_DIR=$prefix" "-DOPENSSL_INCLUDE_DIR=$prefix/include" \
    "-DOPENSSL_SSL_LIBRARY=$prefix/lib/libssl.a" "-DOPENSSL_CRYPTO_LIBRARY=$prefix/lib/libcrypto.a" \
    -DCURL_ZLIB=ON "-DZLIB_INCLUDE_DIR=$prefix/include" "-DZLIB_LIBRARY=$prefix/lib/libz.a" \
    -DHTTP_ONLY=OFF -DCURL_DISABLE_HTTP=OFF -DCURL_DISABLE_FTP=OFF -DCURL_ENABLE_SMB=OFF \
    -DCURL_USE_LIBPSL=OFF -DCURL_USE_LIBSSH2=OFF -DCURL_USE_LIBSSH=OFF -DCURL_USE_GSSAPI=OFF \
    -DUSE_LIBIDN2=OFF -DCURL_BROTLI=OFF -DCURL_ZSTD=OFF -DUSE_NGHTTP2=OFF \
    -DUSE_NGTCP2=OFF -DUSE_QUICHE=OFF "${args[@]}" "${extra[@]}"
cmake --build "$out/build" --parallel 2
"$zig" ar t "$out/build/lib/libcurl.a" | sed 's|.*/||; s/\.c\.o$/.o/' | sort > "$out/oracle-members"
"$zig" ar t "$prefix/lib/libcurl.a" | sed 's|.*/||' | sort > "$out/direct-members"
diff -u "$out/oracle-members" "$out/direct-members"
# Public symbols, excluding compiler-generated/private symbols and runtime helpers.
nm_flags=(-g --defined-only)
if [[ $(uname -s) == Darwin ]]; then nm_flags=(-gU); fi
for kind in oracle direct; do
    archive="$prefix/lib/libcurl.a"
    [[ $kind == direct ]] || archive="$out/build/lib/libcurl.a"
    nm "${nm_flags[@]}" "$archive" | awk 'NF == 3 {name=$NF; sub(/^_curl_/, "curl_", name); if (name ~ /^curl_/) print name}' | sort -u > "$out/$kind-symbols"
    test -s "$out/$kind-symbols"
done
diff -u "$out/oracle-symbols" "$out/direct-symbols"
printf 'Archive membership and public symbols match. Review %s/build/lib/curl_config.h\n' "$out"
