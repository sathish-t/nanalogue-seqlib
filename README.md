[![CI](https://github.com/sathish-t/nanalogue-seqlib/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/sathish-t/nanalogue-seqlib/actions/workflows/rust.yml)

# nanalogue-seqlib

An experimental fork of [rust-bio/rust-htslib](https://github.com/rust-bio/rust-htslib), providing HTSlib bindings and a high-level Rust API for SAM/BAM/CRAM files. The main goal is to evaluate whether this fork can replace upstream `rust-htslib` in [Nanalogue](https://github.com/DNAReplicationLab/nanalogue).

The Cargo package and Rust import names remain `rust-htslib` and `rust_htslib`. This is not a claim of complete upstream API compatibility: the fork has a reduced API and safety-related changes. A crates.io dependency on `rust-htslib` still selects upstream, not this repository.

Clone the fork:

```shell
git clone https://github.com/sathish-t/nanalogue-seqlib.git
```

## Requirements

Install Rust, a C toolchain compatible with the `cc` crate, Clang/libclang for bindgen, and CMake for feature combinations that build compression dependencies from source. The `hts-sys` dependency builds HTSlib; this repository does not require an HTSlib submodule.

**Bindgen is mandatory in this fork**, including with `--no-default-features`. The `bindgen` Cargo feature remains as a compatibility alias, not an opt-in switch. Pre-built upstream bindings do not replace this build requirement.

## Evaluate with Nanalogue

For sibling checkouts named `nanalogue` and `nanalogue-seqlib`, replace Nanalogue's existing `rust-htslib` dependency in its local `Cargo.toml` with:

```toml
[dependencies]
rust-htslib = { path = "../nanalogue-seqlib", features = ["libdeflate"] }
```

Use the path to the checkout containing the changes you want to evaluate. A `[patch.crates-io]` entry alone will not override an exact `=1.0.0` dependency with this fork's `1.0.2`; replacing the dependency avoids that version mismatch.

From the Nanalogue checkout, verify resolution and compatibility:

```shell
cargo tree -i rust-htslib
cargo test --all-features
```

The tree must show your local fork path. Keep the downstream manifest and lockfile changes local while evaluating. Passing these tests is evidence for that pair of revisions, not a guarantee of compatibility with all upstream users or future Nanalogue versions.

## Features and development

Default features enable bzip2, lzma, and HTTP access through curl. If you do not need these capabilities, disable default features (bindgen still runs):

```toml
[dependencies]
rust-htslib = { path = "../nanalogue-seqlib", default-features = false }
```

The `s3` and `gcs` features enable the corresponding HTSlib remote-storage support; `libdeflate` enables libdeflate compression support.

Run the fork's checks and generate API documentation locally:

```shell
cargo fmt -- --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo doc --all-features --no-deps
```

CI is configured for native Linux x86_64/ARM64 feature tests and macOS Intel/Apple Silicon all-feature tests. It does not currently exercise MUSL. [Upstream API documentation](https://docs.rs/rust-htslib) is useful background but may differ from this fork; use locally generated documentation for its current API.

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

Licensed under the MIT license https://opensource.org/licenses/MIT. This project may not be copied, modified, or distributed except according to those terms.
Some test files are taken from https://github.com/samtools/htslib.
