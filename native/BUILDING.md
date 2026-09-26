# Native build maintenance

The native libraries are bundled as source under `vendor/` rather than taken
from system installations. They serve four main purposes:

| Category | Libraries | Purpose |
| --- | --- | --- |
| Genomic file I/O | HTSlib | Reads and writes sequencing formats such as SAM, BAM and CRAM. |
| Compression | zlib, bzip2, XZ's liblzma, libdeflate | Provide general-purpose compression and decompression used by HTSlib. |
| Genomic compression | htscodecs | Provides specialized codecs for CRAM; vendored inside the HTSlib tree. |
| Networking and security | curl (libcurl), OpenSSL | curl retrieves remote data over HTTP/HTTPS/FTP/FTPS; OpenSSL provides TLS and cryptographic primitives. |

This guide covers target ABIs, generated inputs and validation for contributors
updating the vendored native stack. For upstream versions, source checksums,
licenses and local patches, see [SOURCES.md](SOURCES.md). For consumer build
requirements, see the [README](../README.md#requirements).

## Goal

This crate and its vendored native libraries build using only
the Rust and Zig toolchains, apart from unavoidable platform SDK and linking
facilities such as the macOS SDK. CMake, Make and Perl are confined to optional
maintainer oracle comparisons; they are not consumer build requirements.

## Current build and target support

Direct Zig compilation uses ReleaseSafe, including curl and OpenSSL.
Perl and OpenSSL Configure/Make run only during maintainer oracle regeneration;
curl's CMake/Make comparison is likewise maintainer-only.
See [golden provenance and callback corrections](openssl/README.md).

Consumer builds accept six targets: x86_64/aarch64 Linux GNU, Linux musl and
macOS. `native/target_spec.rs`, shared by Cargo, validation tools and bindgen,
also models the historical artifact matrix below. Modeling a target does not
enable it or establish runtime compatibility.

## Build files and responsibilities

Paths below are relative to the repository root. Keep `build.rs` and `build.zig`
as the conventional Cargo and Zig entry points; their roles are broader than
building compression libraries alone.

`build.rs` coordinates the following sequence for a network-enabled Cargo
build. Every native build stage is a Zig build command.

```diagram
             ┌──────────────────────────┐
             │ native/target_spec.rs    │
             │ Target and ABI mappings  │
             └────────────┬─────────────┘
                          ▼
             ┌──────────────────────────┐
             │ build.rs                 │
             │ Coordinates these steps  │
             └────────────┬─────────────┘
                          │
                          ▼
     1. build.zig: compression
                          │
                          ▼
     2. build.zig: openssl
          └─ delegates to native/openssl.zig
                          │
                          ▼
     3. build.zig: curl
          └─ delegates to native/curl.zig
                          │
                          ▼
     4. build.zig: htslib
                          │
                          ▼
     5. Emit Cargo's final linking settings
```

Steps 2 and 3 are skipped when the `curl` feature is disabled. The curl build
uses the OpenSSL and zlib headers/static archives installed by the preceding
Zig steps in the same `OUT_DIR/native` prefix.

`native/target_spec.rs` is Rust code, not a module imported by Zig. `build.rs`
uses it to select Zig target/CPU arguments. `build.zig`
receives target, CPU and feature settings through command-line options.
Within that Zig build, `native/openssl.zig` maps the resolved OS/architecture
to an OpenSSL configuration; the maintainer oracle uses the corresponding
Rust mapping. Target metadata describes what to build, not the build sequence.

| File | Responsibility | Naming rationale |
| --- | --- | --- |
| `build.rs` | Coordinates native build stages, selects bindings and emits Cargo linker settings. | Conventional Cargo build-script entry point. |
| `build.zig` | Defines the public Zig build commands and resolves target/options. Delegates OpenSSL build construction to `native/openssl.zig`. | Conventional Zig build entry point; not limited to compression. |
| `native/openssl.zig` | Implements OpenSSL configuration, source selection, ReleaseSafe compilation, header/archive installation and focused test commands. | Imported implementation of the TLS-library build, not a separate build entry point. |
| `native/curl.zig` | Builds curl from `curl-sources.zig` and `curl_config.h`, using the previously installed OpenSSL/zlib headers. | Owns curl compilation and static/header installation. |
| `native/target_spec.rs` | Maps Rust triples to Zig targets/CPU settings; records ABI requirements, artifact floors and binding availability. | Describes target requirements rather than performing a build. |
| `tests/native_integration.rs` | Tests C/Rust layouts, feature wiring, library versions, providers and HTTP access. | Covers native integration, not just ABI layout. |

## Targets and compatibility baselines

A build produces a binary for a particular operating system and processor,
called its **target**. The machine running the build, called the **runner** in
CI, need not be the machine that will run that binary. For example, building
on a recent Ubuntu runner does not by itself make the result usable on older
Linux systems.

An **artifact** is a packaged build output. Its **compatibility baseline** says
which systems it is intended to run on. For Linux, this includes the system's
C runtime library: glibc (the GNU variant) or musl. A manylinux 2.17 baseline
means targeting systems with glibc 2.17 or later, subject to the other platform
requirements; it is not the version of Ubuntu used to build it.

The table describes the historical release combinations we want the build
design to accommodate, not a list of targets already enabled in this crate.
The ABI notes refer to the rules compiled Rust and C code must agree on, such
as how functions exchange arguments and how data is laid out in memory.

| Artifact family | Architectures | ABI considerations |
| --- | --- | --- |
| macOS | x86_64, aarch64 | Rust deployment target and Apple SDK remain authoritative |
| manylinux 2.17 | x86_64, aarch64, armv7 | ARMv7 hard-float (`gnueabihf`), 32-bit pointers |
| manylinux 2.28 | x86_64, aarch64 | Same native configurations as 2.17; different glibc symbol floor |
| manylinux 2.29 | powerpc64le, riscv64gc | Little-endian; RISC-V requires M/A/F/D/C, Zicsr and Zifencei |
| manylinux 2.34 | x86_64, aarch64 | Same native configurations as 2.17; different glibc symbol floor |
| musllinux 1.2 | x86_64, aarch64, arm, powerpc64le | No glibc suffix; `arm` uses ARMv6-class hard-float (`musleabihf`) |

**Only six targets are currently enabled:** x86_64/aarch64 Linux GNU, Linux
musl and macOS. The table represents 15 release variants but only 11 distinct
Rust target names, because several variants differ only in their minimum
system-library version. The five additional targets—ARMv7 GNU, ARM musl,
PowerPC64LE GNU/musl and RISC-V64GC GNU—still need their own Rust declarations
for the C libraries (bindings) and compatibility tests before they can be used.

The target model records the intended minimum versions, but ordinary Cargo
builds do not yet apply the manylinux baselines from this table. A release must
also use compatible system headers and libraries when linking Rust code, then
check that the finished binary does not require newer system-library functions.
Choosing a target name alone is not proof of compatibility.

The historical combinations come from DNAReplicationLab/nanalogue's v0.1.11
release workflow at
[`bcd543f`](https://github.com/DNAReplicationLab/nanalogue/commit/bcd543fd430b653143707878f13b802b2e0c6b04).

<details>
<summary>Historical target names and ABI details (enabled and not-yet-enabled targets)</summary>

This section covers the full modeled matrix: both the six enabled targets and
the five not-yet-enabled targets listed above. The ARM, RISC-V and PowerPC
target-name examples below refer to not-yet-enabled targets; the byte-order,
data-layout and musllinux notes cover the matrix more broadly.

These details guide changes to the mappings in `native/target_spec.rs`; they
do not establish support for the additional targets. The workflow evidence
records how historical releases were configured, not an inspection of the
released binaries themselves.

The historical workflow pairs `rust_target: arm-unknown-linux-musleabihf` with
`compat_tag: musllinux_1_2_arm` and passes that Rust target unchanged to
`cargo zigbuild`. Hard-float is encoded directly by `eabihf`; ARMv6-class is
inferred from Rust target semantics, not an explicit workflow CPU flag.
The same workflow specifies GNU floors through target suffixes, including
`armv7-unknown-linux-gnueabihf.2.17`, `riscv64gc-unknown-linux-gnu.2.29`, and
`powerpc64le-unknown-linux-gnu.2.29`. Soft-float and ARMv7 musl are distinct Rust
targets, not interchangeable names for the historical ARM musl target.

All modeled targets are little-endian (least-significant byte first). ARM uses
ILP32 (32-bit C integers, longs and pointers); the others use LP64 (32-bit C
integers, 64-bit longs and pointers). The binding generator accepts all their
descriptors, but Cargo rejects targets without approved bindings.

The musllinux 1.2 label describes an artifact's compatibility requirements; it
does not imply that Zig bundles exactly musl version 1.2.0.

</details>

## Regenerate bindings after header or toolchain changes

Rust needs declarations describing the C functions and data structures it uses
from HTSlib. These **bindings** are generated Rust files stored in
`native/bindings/`. They must agree with the C code actually compiled into the
library; otherwise, for example, Rust could read a field at the wrong place
in memory.

Two kinds of changes can make those declarations out of date:

* **Header changes:** C `.h` files declare functions and data structures.
  Updating vendored HTSlib or editing `native/wrapper.h` may change declarations
  seen by the binding generator. This concerns the C interface, not just any
  edit to the library's implementation.
* **Toolchain changes:** a toolchain is the set of compiler and related build
  tools, not a chain of software-package dependencies. For these bindings,
  relevant tools include Zig, which supplies the C compiler and target system
  headers, and bindgen/libclang, which read C declarations and generate Rust.
  Updating them can change the generated declarations or target layouts.

After relevant changes, regenerate and review the bindings. An unrelated Rust
dependency update does not by itself require regeneration.

Run maintainer commands from the repository root. Binding generation uses the
separate `native/bindgen` Cargo package and requires libclang; consumer builds
use checked-in bindings and do not run it.

```sh
cargo run --manifest-path native/bindgen/Cargo.toml -- x86_64-unknown-linux-gnu
```

Regenerate all six supported targets after relevant changes, then run
the native ABI and format/codec tests on each target. Keep bindings separate
by target ABI; do not substitute an existing 64-bit file for a new target.

## Regenerate and check source inventories

Bindings describe how Rust calls C; a **source inventory** describes which C
files must be compiled and with what settings. A new curl or OpenSSL release
can add, remove or rename files, or change headers and source files generated
from templates. A direct Zig build needs to reproduce those choices rather
than silently keep using a list from an older release.

Here, **regeneration** means deriving that information again from the vendored
sources and their build configuration. Do this when updating those libraries
or changing their build options, not for every ordinary Cargo build. The curl
tool updates a checked-in source list; the OpenSSL tool writes reference files
to a disposable directory so maintainers can inspect and compare them.

The inventory tools use Zig 0.15.2 and are not called by `build.rs` or
`build.zig`. The OpenSSL inspection tool additionally requires Perl and Make
to run the pinned upstream build system as a reference, or *oracle*.

```sh
# Check curl's manifest without writing it; omit --check to regenerate.
zig run native/generate-curl-sources.zig -- --check

# Check the inspector's comparison logic.
zig test native/inspect-openssl.zig

# Generate OpenSSL reference inputs. The output directory must not exist.
zig run native/inspect-openssl.zig -- target/openssl-oracle
# Append --archives <oracle-install>/lib to compare x86_64 Linux archive members.

# Compare all checked-in golden inputs with that fresh oracle.
zig run native/materialize-openssl.zig -- target/openssl-oracle --check
# Omit --check to regenerate; inspect the resulting diff.
```

`native/curl-sources.zig` lists all 196 C sources from curl 8.22.0's authoritative
`vendor/curl/lib/Makefile.inc`, including conditionally empty protocol/backend
files. Its header records the input SHA-256. The generator accepts only the
pinned literal lists and `CSOURCES` references; it is not a Make interpreter.
Use the Zig command above rather than the historical generator filename in
the manifest's attribution comment. `native/curl.zig` consumes this manifest.

The OpenSSL inspector runs Configure with
`no-shared no-tests no-module no-dso no-engine no-asm -fPIC`. It generates
library C/header inputs without compiling C, sanitizes inherited build flags,
and fixes the installation prefix and epoch for reproducible metadata.
It reads `configdata.pm`, follows internal provider-library dependencies and
preserves object ownership, defines and include ordering. Each configuration
gets a `manifest.json` containing generated-file SHA-256 hashes and an
`oracle.log`; `comparison.json` summarizes differences. Outputs remain in the
supplied disposable directory, not the vendor tree.

## Running the upstream OpenSSL test suite

The optional upstream suite checks compatibility of the corrected sources
with OpenSSL's own tests. Configure uses its normal `-O3` optimization here:
this does **not** reproduce `build.zig` ReleaseSafe or replace its callback,
provider and TLS tests. Do not weaken ReleaseSafe checks to make tests pass.
Perl and the OpenSSL Make harness are maintainer-only requirements for this
optional suite, not requirements of the direct OpenSSL consumer build.
The separate curl oracle also needs CMake/Make; consumer builds do not.

`vendor/openssl` intentionally omits test/fuzz harness sources. Restore those
from the exact pinned OpenSSL 3.6.4 upstream commit, then overlay the corrected
vendored files **onto the disposable copy**, never in the reverse direction.
Run this Bash snippet from the repository root on x86_64 Linux with Zig 0.15.2,
Perl, Make, curl, tar, GNU coreutils, `file`, and `/usr/bin/time` installed.
It verifies the archive before extraction, creates fresh source/build/cache
directories under `target`, and runs GNU first, then actual musl binaries.
Do not track the restored harness, wrappers, caches or logs.

```bash
(
set -euo pipefail
root=$PWD
zig=$(realpath "$(command -v zig)")
test "$("$zig" version)" = 0.15.2
mkdir -p "$root/target"
run=$(mktemp -d "$root/target/openssl-upstream.XXXXXX")
printf 'Sources, builds and logs: %s\n' "$run"
commit=d3c1b1169b3569ff3069e5b399f47b2b28e03d79
sha=f4e3080732a86b21e2220cf31c3c60026e923d6193bf371ba872e71fb13bb7c7
curl --fail --location --retry 2 \
  "https://codeload.github.com/openssl/openssl/tar.gz/$commit" \
  --output "$run/upstream.tar.gz"
printf '%s  %s\n' "$sha" "$run/upstream.tar.gz" | sha256sum --check --strict -
mkdir "$run/src"
tar -xzf "$run/upstream.tar.gz" --strip-components=1 -C "$run/src"
cp -a "$root/vendor/openssl/." "$run/src/"

for abi in gnu musl; do
  cat > "$run/cc-$abi" <<EOF
#!/bin/sh
exec "$zig" cc -target x86_64-linux-$abi -mcpu=baseline "\$@"
EOF
done
cat > "$run/ar" <<EOF
#!/bin/sh
exec "$zig" ar "\$@"
EOF
cat > "$run/ranlib" <<EOF
#!/bin/sh
exec "$zig" ar s "\$@"
EOF
chmod +x "$run/cc-gnu" "$run/cc-musl" "$run/ar" "$run/ranlib"

for abi in gnu musl; do (
  build="$run/build-$abi"
  mkdir "$build"
  cd "$build"
  exec > >(tee commands.log) 2>&1
  set -x
  unset CFLAGS CPPFLAGS CXXFLAGS LDFLAGS CROSS_COMPILE
  export SOURCE_DATE_EPOCH=0
  export ZIG_LOCAL_CACHE_DIR="$build/zig-local-cache"
  export ZIG_GLOBAL_CACHE_DIR="$build/zig-global-cache"
  export CC="$run/cc-$abi" AR="$run/ar" RANLIB="$run/ranlib"
  extra=()
  if [[ "$abi" == musl ]]; then
    export CC=cc-musl AR=ar RANLIB=ranlib
    extra=("--cross-compile-prefix=$run/")
  fi
  date -u
  /usr/bin/time -p -o configure.time perl "$run/src/Configure" \
    linux-x86_64 --prefix=/native --libdir=lib --openssldir=/etc/ssl \
    no-shared no-asm no-module no-dso no-engine -fPIC "${extra[@]}" \
    > configure.log 2>&1
  perl configdata.pm --dump > configdata.log
  /usr/bin/time -p -o build.time make -j4 > build.log 2>&1
  file apps/openssl > executable.log
  apps/openssl version -a >> executable.log
  export HARNESS_TAP_COPY="$build/tests.tap"
  if /usr/bin/time -p -o test.time make test HARNESS_JOBS=4 > test.log 2>&1; then
    result=0
  else
    result=$?
  fi
  echo "$result" > test.exit
  tail -45 test.log
  date -u
  exit "$result"
); done
)
```

Inspect `configdata.log`, compiler commands in `build.log`, and `executable.log`
to confirm the intended target. Musl must be declared through `CROSS_COMPILE`
(the Configure prefix above), even though its x86_64 binaries run on this host.
Otherwise `02-test_errstr` compares musl's `strerror()` strings against the
host Perl's glibc strings and reports false failures. Upstream skips that
recipe and `04-test_conf` for declared cross builds; do not patch their
expectations or substitute GNU executables. In the initial undeclared-cross
musl run, only `02-test_errstr` failed (73 assertions); a standalone musl libc
probe reproduced all 73 strings without OpenSSL. `04-test_conf` passed there.

Reference runs on **2026-09-23 UTC**, OpenSSL **3.6.4** with the pinned snapshot,
local callback corrections, Zig **0.15.2**, baseline x86_64 CPUs and the options
above produced the following results. Counts exclude the separate skipped
FIPS preparation recipe; recipe counts and top-level TAP assertion counts
are different units and must not be added together.

| Executed target | Recipes: pass / fail / skip | TAP assertions: pass / fail / skip | Harness total assertions |
| --- | --- | --- | --- |
| x86_64 Linux GNU | 303 / 0 / 52 | 4014 / 0 / 65 | 4079 |
| x86_64 Linux musl, declared cross build | 301 / 0 / 54 | 3870 / 0 / 66 | 3936 |

Expected skips include FIPS, shared-library/dynamic-engine tests, other
disabled or default-off features, external integrations and absent fuzz
corpora. Preserve exact reasons in `test.log`/`tests.tap`; these results do not
validate skipped features, other architectures, or the direct ReleaseSafe
build. Keep the independent direct tests, for example:

```sh
zig build test-openssl -Dtarget=x86_64-linux-gnu -Dcpu=baseline
zig build test-openssl -Dtarget=x86_64-linux-musl -Dcpu=baseline
```

## OpenSSL configuration differences to preserve

For pinned OpenSSL 3.6.4 with these options, the seven Configure targets share
997 libcrypto objects, 94 libssl objects, object settings, compile groups and
disabled features. The 28 provider-common objects enter through internal
library dependencies; walking only `sources[libcrypto]` misses them.

Of 120 generated C/header/include-fragment files, 116 are identical across these configurations.
The inventory includes `providers/implementations/include/prov/blake2_params.inc`,
which the original C/header-only oracle filter omitted.
The remaining files encode the following differences:

| Generated file | Configuration dimension |
| --- | --- |
| `crypto/buildinf.h` | Target name and compiler flags |
| `include/crypto/bn_conf.h` | 32/64-bit words |
| `include/crypto/dso_conf.h` | OS library extension, even with no-dso |
| `include/openssl/configuration.h` | macOS macro, word size and RC4 integer type |

`RC4_INT` is unsigned int on Linux x86_64 and macOS, but unsigned char on the
other modeled Linux architectures. Do not infer this choice from pointer width.
Global flags also differ: `OPENSSL_USE_NODELETE` on Linux, explicit `L_ENDIAN`
on x86_64/PPC64LE/macOS, and `-latomic` in the ARM oracle. ARM atomic linkage
must use appropriate target runtime facilities, never a discovered host library.
Recheck these inventories and differences whenever the pinned release or
Configure options change.

## Validate native build changes before enabling targets

Compiling successfully is only the first check. Rust and C might still disagree
on a structure's layout, an optional feature might be missing, or a library
might fail when used. The checks below test both the target descriptions and
the behavior of the compiled libraries.

Run them after changing the native build, regenerating bindings or updating a
vendored library. A test run on one platform does not establish support for
another: checking an ARM target's compiler settings on an x86_64 machine is
not the same as running the resulting library on ARM.

```sh
mkdir -p target
rustc --edition=2018 --test native/target_spec.rs -o target/native-target-tests
target/native-target-tests --include-ignored
cargo test --all-features --test native_integration -- --include-ignored
cargo test --no-default-features --test native_integration
```

The standalone target tests ask Zig to report settings such as pointer size
and required processor features for all 15 release combinations. They do not
build or run a program for each target. The native integration tests exercise
the libraries on the target where the tests run: checking Rust/C layouts,
feature selection, versions, OpenSSL providers and HTTP access where enabled.
Also run the wider file-format and compression tests on each intended target
before claiming support for it.

Changes to the network build must preserve static linking, built-in OpenSSL
providers, installed public headers, and HTSlib's HMAC/S3/GCS support. Preserve
curl's FTP/FTPS/HTTP/HTTPS protocols, threaded resolver, zlib/OpenSSL dependencies
and disabled optional dependency set. Compare GNU/musl/macOS configurations,
especially `strerror_r`, time/off_t widths, Unix APIs and Apple SecTrust.
Test CA overrides, Linux runtime bundle discovery, HTTPS proxies and S3 without
embedding build-machine CA paths or discovering host libraries. When moving C
code to ReleaseSafe, investigate sanitizer failures rather than disabling checks.

## curl capability contract and comparison

`native/curl_config.h` replaces configure-time probing. It is maintained source,
not a header copied blindly from the build host. Undefined means disabled;
do not write `#define HAVE_FOO 0` for curl's `#ifdef` capabilities.
`native/curl.zig` supplies `HAVE_CONFIG_H`, `BUILDING_LIBCURL`, hidden visibility,
Linux `_GNU_SOURCE`, and the GNU libc discriminator. `USE_OPENSSL` and `HAVE_LIBZ`
come from the contract. Headers come only from the vendor tree and the same
target's installed direct OpenSSL/zlib prefix. No CA file/path is embedded.

The initial audit compares the prior x86_64 GNU CMake output, fresh x86_64 musl
and ARMv7 GNU 2.17 CMake outputs, curl's pinned `CMake/unix-cache.cmake`, and Zig
0.15.2's target headers. macOS capability choices follow those target sources;
they still require Apple SDK compile/link and runtime verification on macOS.

| Dimension | Contract |
| --- | --- |
| OS-independent Unix | pthread resolver, IPv4/IPv6, poll, sockets/socketpair, reentrant time functions, `getaddrinfo`, `sockaddr_storage`, `timeval`, OpenSSL and zlib |
| Linux | `_GNU_SOURCE`; accept4, pipe2, eventfd, sendmmsg, memrchr, six-argument gethostbyname_r, five-argument fsetxattr |
| GNU libc | Pointer-returning strerror_r; time_t follows C long (32-bit on ARMv7) |
| musl 1.2 | Integer-returning POSIX strerror_r; 64-bit time_t including ARM |
| macOS | POSIX strerror_r, six-argument fsetxattr, Mach time, filio/sockio headers, Apple SecTrust; Linux-only APIs remain undefined |
| Word size | Compiler-derived long/size_t; 64-bit off_t (`_FILE_OFFSET_BITS=64`) and curl_off_t; int/socket are 32-bit |

The GNU floors share capabilities: the selected Linux APIs predate glibc 2.17.
Zig's explicit target versions constrain symbols rather than copying per-floor
configurations. musl CMake additionally detects `stropts.h`; the direct build
intentionally omits that unused legacy STREAMS include. GNU/ARM differences
are only long/size_t/time_t widths. Apple headers gate `memset_s` on Annex K
opt-in, so unlike the upstream Unix cache's assumption, it stays undefined;
curl retains its portable secure-zero fallback. Native compile-time assertions check actual
headers against this contract, including both strerror_r ABIs. The configured
protocol list is exactly FTP, FTPS, HTTP, HTTPS. The pinned source identifies
itself as **8.22.0-DEV**, numeric **0x081600**; no version file is rewritten.

Commands below run from the repository root. They are maintainer checks, not
consumer dependencies. The matrix tool uses `target_spec.rs` rather than a
second list of targets. It links every modeled Linux variant, including HTSlib,
curl, OpenSSL and all compression libraries;
only x86_64 GNU/musl executables run on an x86_64 Linux host. Other architectures
remain compile/link evidence, not runtime or Rust binding approval.

```sh
zig run native/generate-curl-sources.zig -- --check
rustc --edition=2018 native/check-curl-targets.rs -o target/check-curl-targets
target/check-curl-targets
# An optional substring argument restricts the matrix, e.g. x86_64-linux-musl.
python3 tests/native_curl.py target/curl-matrix/x86_64-linux-gnu.2.17/curl-test
python3 tests/native_curl.py target/curl-matrix/x86_64-linux-musl/curl-test

# Optional CMake/Make oracle: output directory must not exist.
# Requires CMake, Make, Bash, nm and standard text tools in addition to Zig.
bash native/inspect-curl.sh target/curl-matrix/x86_64-linux-musl \
  x86_64-linux-musl baseline target/curl-oracle-musl

# Actual HTSlib CA/proxy/S3/GCS requests, with local certificate fixtures.
# Requires Python and the openssl executable for fixtures, never for builds.
cargo test --all-features --test native_integration -- --include-ignored
python3 tests/native_tls.py <all-features-OUT_DIR>/native x86_64-linux-gnu
```

The oracle checks the 196 normalized archive members and public `curl_*`
symbols. Review its `build/lib/curl_config.h` and compiler flags separately.
Compiler helper/private symbol differences from ReleaseSafe versus CMake's
Release optimization are not public API differences. The transfer fixture
checks localhost DNS, IPv6, redirects, ranges, auth, cookies, gzip, certificate
and hostname rejection, per-request CA precedence, FTP, explicit and implicit
FTPS. The C probe also checks poll/wakeup across pthreads, versions, dependency
flags, protocols and strerror output. The TLS fixture independently checks S3
SigV4 and GCS authorization/requester-pays through the actual HTSlib backend.

The S3 test initially trapped at curl's write-callback invocation. HTSlib's
response callback took `void *` rather than curl's required `char *`. Correcting
that signature and the analogous header/upload callbacks in the authoritative
HTSlib sources fixes the undefined call; no curl source or sanitizer setting
is changed. Preserve these three vendor edits as a separable correction.
