[![CI](https://github.com/sathish-t/nanalogue-seqlib/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/sathish-t/nanalogue-seqlib/actions/workflows/rust.yml)

# nanalogue-seqlib

An experimental fork of [rust-bio/rust-htslib](https://github.com/rust-bio/rust-htslib), providing HTSlib bindings and a high-level Rust API for SAM/BAM/CRAM files. The main goal is to evaluate whether this fork can replace upstream `rust-htslib` in [Nanalogue](https://github.com/DNAReplicationLab/nanalogue).

The Cargo package and Rust import names remain `rust-htslib` and `rust_htslib`. This is not a claim of complete upstream API compatibility: the fork has a reduced API and safety-related changes. A crates.io dependency on `rust-htslib` still selects upstream, not this repository.

Clone the fork:

```shell
git clone https://github.com/sathish-t/nanalogue-seqlib.git
```

## Requirements

Install Rust and **Zig 0.15.2**. Default/network-enabled builds also require
CMake (3.18+), Make and Perl. On macOS, install the Apple command-line tools/SDK
for native OS headers and final linking. The supported hosts are Linux and macOS; supported targets
are x86_64 and ARM64 Linux GNU/musl and macOS.

```sh
# Optional installer; requires curl, minisign, and xz/tar.
bash native/install-zig.sh
export PATH="$HOME/.local/bin:$PATH"
cargo test --all-features
```

The installer downloads only from Zig's community mirrors and verifies both
the pinned SHA-256 and Zig Software Foundation minisign signature before use.

Set `ZIG` to an absolute executable path if Zig is not on PATH. All native C
compilation and archive creation use Zig, including when cross-compiling.
The final Rust executable still needs a linker for its target; cross-target
users must configure Cargo's target linker separately.

**HTSlib, htscodecs, zlib, bzip2, liblzma, libdeflate, curl and OpenSSL are
copied into `vendor/` and linked statically.** No native `*-sys` crate, `cc`
crate, build-time bindgen/libclang, pkg-config lookup, system installation of
these libraries, submodule checkout or native-source download is used. There
are no Rust build dependencies. Unrelated Rust dependencies still use Cargo.
The `bindgen` and `static` features remain no-op compatibility aliases.

See [native source provenance and licenses](native/SOURCES.md) for versions,
upstream commits, input checksums, local patches and binding regeneration.
The bindings are checked in per target ABI and checked against compiled C
layouts by integration tests.

HTTPS retains certificate and hostname verification without changing the
process environment. `CURL_CA_BUNDLE` is HTSlib's explicit per-request
override; OpenSSL's `SSL_CERT_FILE` and `SSL_CERT_DIR` overrides are also
preserved. Without an override, macOS uses Apple SecTrust and Linux selects the
first readable CA bundle from conventional distribution and OpenSSL locations
at runtime. OpenSSL's `/etc/ssl` defaults remain the final fallback. The Linux
bundle also applies to HTTPS proxies and S3 requests. No build-machine
certificate path is auto-detected; cloud credentials and CRAM reference
sequences remain application/runtime inputs.

## Evaluate with Nanalogue

For sibling checkouts named `nanalogue` and `nanalogue-seqlib`, replace Nanalogue's existing `rust-htslib` dependency in its local `Cargo.toml` with:

```toml
[dependencies]
rust-htslib = { path = "../nanalogue-seqlib", features = ["libdeflate"] }
```

Use the path to the checkout containing the changes you want to evaluate. A `[patch.crates-io]` entry alone will not override an exact `=1.0.0` dependency with this fork's `1.0.2`; replacing the dependency avoids that version mismatch.

To evaluate the pushed vendoring branch instead, replace the dependency with:

```toml
rust-htslib = { git = "https://github.com/sathish-t/nanalogue-seqlib", branch = "vendoring", features = ["libdeflate"] }
```

For reproducibility, replace `branch` with `rev` and the full tested commit.

From the Nanalogue checkout, verify resolution and compatibility:

```shell
cargo tree -i rust-htslib
cargo test --all-features
```

The tree must show your local fork path. Keep the downstream manifest and lockfile changes local while evaluating. Passing these tests is evidence for that pair of revisions, not a guarantee of compatibility with all upstream users or future Nanalogue versions.

## Features and development

Default features enable bzip2, lzma, and HTTP/HTTPS/FTP access through curl.
If you do not need these capabilities, disable default features. This smaller
build requires only Rust and Zig (plus a final Rust target linker), not
CMake/Make/Perl:

```toml
[dependencies]
rust-htslib = { path = "../nanalogue-seqlib", default-features = false }
```

The `s3` and `gcs` features enable the corresponding HTSlib remote-storage support;
`libdeflate` enables libdeflate compression support. Disabling bzip2 or lzma
reduces CRAM codec compatibility. Runtime HTSlib plugins are disabled; enabled
network handlers are compiled in.

Run the fork's checks and generate API documentation locally:

```shell
cargo fmt -- --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo doc --all-features --no-deps
```

CI is configured to build and run seqlib's default-feature, `--no-default-features`, and `--all-features` test configurations on GNU/Linux and MUSL for both x86_64 and ARM64, plus macOS Intel and Apple Silicon. MUSL jobs use matching-architecture runners and also target MUSL in the compile-fail tests' nested Cargo invocations. [Upstream API documentation](https://docs.rs/rust-htslib) is useful background but may differ from this fork; use locally generated documentation for its current API.

# Alternatives

There's [noodles](https://github.com/zaeleus/noodles) by [Michael Macias](https://github.com/zaeleus) which implements a large part of htslib's C functionality in pure Rust (still experimental though).

# Upstream authors

* [Johannes Köster](https://github.com/johanneskoester)
* [Christopher Schröder](https://github.com/christopher-schroeder)
* [Patrick Marks](https://github.com/pmarks)
* [David Lähnemann](https://github.com/dlaehnemann)
* [Manuel Holtgrewe](https://github.com/holtgrewe)
* [Julian Gehring](https://github.com/juliangehring)

For other contributors, see [here](https://github.com/rust-bio/rust-htslib/graphs/contributors).

## License

The Rust wrapper is licensed under the MIT license https://opensource.org/licenses/MIT.
Bundled native code retains its own licenses and notices; see
[native/SOURCES.md](native/SOURCES.md). Redistributors must retain the applicable
notices; the entire vendor tree is not exclusively MIT-licensed.
Some test files are taken from https://github.com/samtools/htslib.
