# OpenSSL golden inputs — direct ReleaseSafe build

These inputs come from the vendored OpenSSL **3.6.4**, upstream commit
`d3c1b1169b3569ff3069e5b399f47b2b28e03d79`. See [source provenance](../SOURCES.md)
for the download checksum and retained Apache-2.0 license. Generated sources
are materialized from the locally corrected vendor templates, never hand-edited.
Ordinary Cargo builds use this tree, not Configure/Perl/Make. curl also compiles
directly with Zig. This directory does not enable additional Rust target bindings.

## Reproduction and layout

From the repository root, with Zig 0.15.2, Perl and Make:

```sh
zig run native/inspect-openssl.zig -- target/openssl-oracle
zig run native/materialize-openssl.zig -- target/openssl-oracle --check
# Omit --check to materialize fresh oracle output after reviewing differences.
zig build openssl -Dtarget=x86_64-linux-gnu -Dcpu=baseline --prefix target/direct-ssl
```

Use an oracle directory exactly two levels below the repository root as above;
upstream embeds relative template paths in generated comments. The inspector
fixes the epoch, prefix (`/native`) and OpenSSL directory (`/etc/ssl`) and clears
inherited build flags. It selects `no-shared no-tests no-module no-dso no-engine
no-asm -fPIC` for all seven configurations.

* `manifest.json` is the complete canonical linux-x86_64 oracle snapshot. Its
  997 libcrypto and 94 libssl object entries preserve source paths, owning
  library, per-object definitions/includes and the five compile groups. It
  includes the 28 provider-common dependency objects.
* `comparison.json` records seven target settings, disabled-feature equality,
  inventory equality and generated-file equivalence classes. Its reference
  `buildinf.h` hashes describe the oracle, **not** direct-build metadata.
* `common/` contains 116 byte-identical C/header/include-fragment inputs.
  The earlier C/header-only inventory missed `prov/blake2_params.inc`.
* `overlays/` contains only eight unique headers: two BN word-size choices,
  two DSO extensions and four `configuration.h` variants. The directory name
  is the first member of each equivalence class, not an exclusive target.
* `../openssl.zig` selects overlays using those equivalence classes and writes
  `crypto/buildinf.h` with honest Zig 0.15.2 ReleaseSafe/no-asm metadata. It
  retains target-specific flags (including RC4 type through configuration.h),
  per-object-before-group include ordering, PIC, threading and built-in
  providers. It installs public headers and `libssl.a`/`libcrypto.a`.

The build uses the caller's resolved Zig target/CPU/libc floor; it does not
discover host libraries. macOS needs `--sysroot` or `SDKROOT` for CommonCrypto
and frameworks, and an explicit deployment-version target matching Rust.
All nine modeled Linux triples compile and link a provider probe with Zig,
including ARM's atomic-runtime references without a host libatomic dependency.
Only x86_64 GNU/musl probes were run; both load default and base providers.
macOS compilation was attempted but blocked by missing Apple SDK headers.
Archive member paths/names follow Zig rather than upstream Make; source
ownership/order remains explicit in the manifest. Linux x86_64 global defined
symbol sets retain the upstream exports with four additional stack adapter
entry points. Archive identity is not expected because Zig names its members
differently and ReleaseSafe generates checked code rather than Configure's -O3.

## Local callback corrections

The initial default/base provider test trapped in `ossl_bsearch` calling
`ossl_provider_cmp` through
`int (*)(const void *, const void *)`, although the actual function takes
`const OSSL_PROVIDER *const *` arguments. ReleaseSafe's indirect-call type
check detects this undefined behavior.

`include/openssl/safestack.h.in` and `util/perl/OpenSSL/stackhash.pm` generate
typed comparator/copy/free adapters. `crypto/stack/stack.c` carries comparator
context through search and allocation-free heapsort; explicit copy/free
adapters cover deep-copy rollback without changing the const source stack.
`include/openssl/stack.h` and `util/libcrypto.num` declare four local additive
entry points. Existing generic functions keep their signatures. Comparator
replacement resets adapter state. No global or thread-local callback state,
platform-specific qsort_r convention, sanitizer suppression or ReleaseFast is used.

TLS exercised further instances of the same incompatible-function-pointer
problem. The additional local corrections are:

* `include/crypto/aes_callbacks.h`, `aes_platform.h`, and `crypto/aes/aes_{cbc,cfb,ofb,wrap}.c`:
  typed AES block/CBC adapters; public AES signatures stay unchanged.
* `include/crypto/sparse_array.h`: synchronous visitor contexts;
  `crypto/evp/evp_local.h` and its do-all callers: typed enumeration adapters;
  `skeymgmt_meth.c`: correctly typed reference/free callbacks.
* `crypto/pem/pem_info.c` and `include/openssl/pem.h`: typed DER/PEM callbacks.
* `providers/implementations/encode_decode/decode_der2key.c.in`: typed decoder
  and key-free adapters in the authoritative generated-source template.
* `providers/implementations/include/prov/digestcommon.h`: provider digest-update
  adapter rather than an incompatible low-level SHA callback.
* `crypto/asn1/tasn_{enc,prn}.c`: invoke mutable and const auxiliary callbacks
  through their actual types, retaining backward-compatible data-pointer semantics.

`native/openssl-stack-test.c` tests duplicate keys, missing-key boundaries,
comparator replacement, nested and concurrent sorts, typed/generated wrapper
families, NULL slots and deep-copy failure at every position with exact free
counts. Run `zig build test-openssl -Dtarget=x86_64-linux-gnu -Dcpu=baseline`.
`tests/native_tls.py` tests local certificate rejection, CA precedence, hashed
CA directories, HTTPS proxying, independently verified S3 signing and GCS
headers through the Cargo-built native libraries. It requires Python and a
system openssl executable only to generate test certificates. No cloud account
or real credentials are used. These tests are not an audit of every OpenSSL API.
