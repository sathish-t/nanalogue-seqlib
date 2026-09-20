# Native source provenance and licensing

All source directories are ordinary checked-in files, not submodules. Nothing
in `build.rs` downloads source. The Cargo crate remains MIT; the bundled code
retains its own notices and licenses below. Preserve those notices when
redistributing sources or binaries. In particular, the whole vendor tree is
not exclusively MIT-licensed.

| Source | Version / upstream release commit | License / notice |
| --- | --- | --- |
| [HTSlib](https://github.com/samtools/htslib) | 1.24, `4b705e4fada8ee2b6b15746f725ee8ac51631803` | MIT and BSD notices: `vendor/htslib/LICENSE` and individual files |
| [htscodecs](https://github.com/samtools/htscodecs) | 1.6.7, `b9fc194f772e45bb0a1f44b08cbf8697a1384bae` | BSD-3-Clause: `vendor/htslib/htscodecs/LICENSE.md` |
| [zlib](https://github.com/madler/zlib) | 1.3.2, `da607da739fa6047df13e66a2af6b8bec7c2a498` | Zlib: `vendor/zlib/LICENSE` |
| [bzip2](https://sourceware.org/git/bzip2.git) | 1.0.8, `6a8690fc8d26c815e798c588f796eabe9d684cf0` | bzip2-1.0.6: `vendor/bzip2/LICENSE` |
| [XZ / liblzma](https://github.com/tukaani-project/xz) | 5.8.4, `d3e650e63c110e830fd5391e7f8b45df0b91d3da` | Compiled liblzma code is 0BSD; see `vendor/xz/COPYING` and `vendor/xz/COPYING.0BSD`. Uncompiled supporting files also carry GPL/LGPL licenses; all supplied notices are retained. |
| [libdeflate](https://github.com/ebiggers/libdeflate) | 1.26, `92e6a0db9fa848d742f9eb286c92afc60f2c3dda` | MIT: `vendor/libdeflate/COPYING` |
| [curl](https://github.com/curl/curl) | 8.21.0, `68720b4837284335b2d63cb358f8f6ce65f5bc55` | curl: `vendor/curl/COPYING`, plus `LICENSES/` |
| [OpenSSL](https://github.com/openssl/openssl) | 3.6.4, `d3c1b1169b3569ff3069e5b399f47b2b28e03d79` | Apache-2.0: `vendor/openssl/LICENSE.txt` |
| [hts-sys wrapper](https://github.com/rust-bio/hts-sys) | 2.2.1, `64f51cc9c649df98d4d85b49c3ce242efe4aa6e6` | MIT: `native/HTS-SYS-LICENSE` |

Release commits identify upstream baselines; the copied crate distributions
can omit upstream files or include generated files. XZ is copied from its
upstream release commit (annotated tag object `9151b328e76bbc468aa64f85c741886b6227cec1`).
Exact crates.io input archives for the remaining sources and XZ's retained
portable `config.h` are:

| crates.io archive | SHA-256 |
| --- | --- |
| hts-sys-2.2.1.crate | fc7e68eb880b02c80cfb41e8dc7904062a3ea7e27b7c4556e88d648dd2f038da |
| libz-sys-1.1.29.crate | 85bc9657773828b90eeb625adff10eeac83cc21bbfd8e23a03eaa8a33c9e28d9 |
| bzip2-sys-0.1.13+1.0.8.crate | 225bff33b2141874fe80d71e07d6eec4f85c5c216453dd96388240f96e1acc14 |
| lzma-sys-0.1.20.crate | 5fda04ab3764e6cde78b9974eec4f779acaba7c4e84b36eca3cf77c581b85d27 |
| libdeflate-sys-1.26.1.crate | d7870e5fbd2766179a937c725fb11f4ca0ef025d982beb61bd3ce755425bd19c |
| curl-sys-0.4.90+curl-8.21.0.crate | 97799a0d220bfb3361e0fe4936966ff8c4b24d65c3f06dfc70d7b680b44e7897 |

OpenSSL 3.6.4 was copied from its upstream release commit using
`https://api.github.com/repos/openssl/openssl/tarball/d3c1b1169b3569ff3069e5b399f47b2b28e03d79`;
download SHA-256: `597c001f956b50243b23384796a1f67267824a48427fdbe1744450e6e13306b2`.
The previous `openssl-src` package's source-file selection was retained;
upstream documentation, demos, tests, and other files not needed to build the
libraries remain omitted.

curl's omitted `docs/` tree was added from its upstream release commit via
`https://api.github.com/repos/curl/curl/tarball/68720b4837284335b2d63cb358f8f6ce65f5bc55`;
download SHA-256: `cf3a1f5f8fbe9ceda62cdf72e3eea215e6a60bc401f7b22e73b195cdd4c08601`.

## Local build choices and patches

* Classic zlib replaces zlib-ng's compatibility implementation. No formats or
  zlib ABI entry points used by HTSlib are removed.
* `vendor/xz/config.h` originated in lzma-sys's portable configuration and is
  retained as the cross-target configuration. Only the liblzma sources are
  compiled; generator programs and size-optimized CRC alternatives are omitted.
* libdeflate's two 512-bit x86 checksum implementations explicitly add
  `evex512` to their function target attributes for Zig 0.15.2 / Clang 20.
  Runtime CPU dispatch remains intact; baseline compilation is not AVX-512.
* HTSlib configuration/version headers are generated in OUT_DIR. LZMA enables
  both `HAVE_LIBLZMA` and `HAVE_LZMA_H`, including on macOS. Plugins stay off.
* The separately maintained htscodecs subtree is at 1.6.7 rather than
  HTSlib 1.24's upstream submodule revision (htscodecs 1.6.6).
* OpenSSL uses portable C (`no-asm`), static built-in providers, no DSO/engine
  loading, and `/etc/ssl` as the default certificate directory. curl uses only
  our static OpenSSL/zlib; unrelated optional native libraries are disabled.
* The original wrapper includes now point into `vendor/htslib`; its C ABI
  size/offset table supports `tests/native_stack.rs`.

## Bindings and updates

`native/bindings/` contains separate files for x86_64/aarch64 Linux GNU, Linux
musl, and macOS. These were generated using bindgen 0.72.1, libclang 14.0.6,
and Zig 0.15.2 target headers. Layout assertions are retained. Consumer builds
do not run bindgen or require libclang; `native/bindgen` is a separate,
maintainer-only Cargo package and is not a dependency of this crate.

To regenerate one target from the repository root:

```sh
cargo run --manifest-path native/bindgen/Cargo.toml -- x86_64-unknown-linux-gnu
```

Regenerate all six after header/toolchain changes, then run the native ABI
tests and format/codec tests on each target. Updating sources also requires
checking upstream build source lists, compiler configuration, optional
dependencies, license changes and security advisories. The version choices
here preserve the previous native dependency baseline rather than claiming
that every bundled release is the newest or free of known vulnerabilities.

Zig itself is a build prerequisite, not copied into the library. The explicit
installer pins 0.15.2 and verifies platform-specific SHA-256 checksums. Its
upstream distribution includes its own compiler/runtime license notices.
