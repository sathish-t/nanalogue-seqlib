# Local OpenSSL callback corrections

These corrections apply to the vendored OpenSSL 3.6.4 source. Paths below
are relative to `vendor/openssl`. Consumer builds still use the upstream
Configure/Make pipeline; source provenance is recorded in `native/SOURCES.md`.

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
counts. After an all-features Cargo build, set `prefix` to its `OUT_DIR/native`
directory and compile the test against that installation:

```sh
zig cc -target x86_64-linux-gnu -mcpu=baseline -O2 \
  -fsanitize=undefined -fsanitize-trap=undefined -I"$prefix/include" \
  native/openssl-stack-test.c "$prefix/lib/libssl.a" "$prefix/lib/libcrypto.a" \
  -pthread -ldl -o target/openssl-stack-test
target/openssl-stack-test
```

`tests/native_tls.py` tests local certificate rejection, CA precedence, hashed
CA directories, HTTPS proxying, independently verified S3 signing and GCS
headers through the Cargo-built native libraries. It requires Python and a
system openssl executable only to generate test certificates. No cloud account
or real credentials are used. These tests are not an audit of every OpenSSL API.
