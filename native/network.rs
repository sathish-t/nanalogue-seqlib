//! Build the vendored TLS and HTTP stack.

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Builds static OpenSSL and curl archives into `out/native`.
///
/// `compiler` and `archiver` are the project-created wrappers around the
/// pinned Zig `cc` and `ar`; in particular, this code never asks CMake or
/// OpenSSL to discover a host compiler or a host copy of either library.
pub fn build(out: &Path, target: &str, compiler: &Path, archiver: &Path) {
    if env::var_os("CARGO_FEATURE_CURL").is_none() {
        return;
    }

    assert!(
        compiler.is_absolute(),
        "C compiler wrapper must be absolute"
    );
    assert!(archiver.is_absolute(), "archiver wrapper must be absolute");

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let prefix = out.join("native");
    let openssl_build = out.join("openssl-build");
    let curl_build = out.join("curl-build");
    let ranlib_wrapper = out.join("zig-ranlib.sh");
    let jobs = env::var("NUM_JOBS").unwrap_or_else(|_| "1".into());
    let (openssl_platform, system, processor) = target_settings(target);

    fresh_dir(&openssl_build);
    fs::create_dir_all(&prefix).expect("create native installation prefix");

    // OpenSSL's generated makefiles use RANLIB as a shell fragment. `ar s`
    // creates the index and keeps every archive operation on the supplied Zig
    // archiver rather than silently finding the host ranlib.
    let ranlib = format!("{} s", shell_quote(archiver));
    fs::write(
        &ranlib_wrapper,
        format!("#!/bin/sh\nexec {} s \"$@\"\n", shell_quote(archiver)),
    )
    .expect("write Zig ranlib wrapper");
    fs::set_permissions(&ranlib_wrapper, fs::Permissions::from_mode(0o755))
        .expect("make Zig ranlib wrapper executable");
    let mut configure = Command::new("perl");
    configure
        .current_dir(&openssl_build)
        .arg(root.join("vendor/openssl/Configure"))
        .arg(openssl_platform)
        .arg(format!("--prefix={}", prefix.display()))
        .arg("--libdir=lib")
        .arg("--openssldir=/etc/ssl")
        .args([
            "no-shared",
            "no-tests",
            "no-module",
            "no-dso",
            "no-engine",
            "no-asm",
            "-fPIC",
        ])
        .env("CC", compiler)
        .env("AR", archiver)
        .env("RANLIB", &ranlib);
    run(&mut configure, "configure OpenSSL");

    let mut make_ssl = Command::new("make");
    make_ssl
        .current_dir(&openssl_build)
        .arg(format!("-j{jobs}"))
        .arg("install_dev")
        .env("CC", compiler)
        .env("AR", archiver)
        .env("RANLIB", &ranlib);
    run(&mut make_ssl, "build and install OpenSSL");

    fresh_dir(&curl_build);
    let mut cmake = Command::new("cmake");
    cmake
        .arg("-S")
        .arg(root.join("vendor/curl"))
        .arg("-B")
        .arg(&curl_build)
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", prefix.display()))
        .arg(format!("-DCMAKE_C_COMPILER={}", compiler.display()))
        .arg(format!("-DCMAKE_AR={}", archiver.display()))
        .arg(format!("-DCMAKE_RANLIB={}", ranlib_wrapper.display()))
        .arg(format!("-DCMAKE_SYSTEM_NAME={system}"))
        .arg(format!("-DCMAKE_SYSTEM_PROCESSOR={processor}"))
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .arg("-DBUILD_SHARED_LIBS=OFF")
        .arg("-DBUILD_CURL_EXE=OFF")
        .arg("-DBUILD_TESTING=OFF")
        .arg("-DBUILD_EXAMPLES=OFF")
        .arg("-DBUILD_LIBCURL_DOCS=OFF")
        .arg("-DBUILD_MISC_DOCS=OFF")
        .arg("-DENABLE_CURL_MANUAL=OFF")
        .arg("-DCURL_USE_PKGCONFIG=OFF")
        .arg("-DCURL_USE_CMAKECONFIG=OFF")
        .arg("-DCURL_CA_BUNDLE=none")
        .arg("-DCURL_CA_PATH=none")
        .arg("-DCURL_USE_OPENSSL=ON")
        .arg(format!("-DOPENSSL_ROOT_DIR={}", prefix.display()))
        .arg(format!(
            "-DOPENSSL_INCLUDE_DIR={}/include",
            prefix.display()
        ))
        .arg(format!(
            "-DOPENSSL_SSL_LIBRARY={}/lib/libssl.a",
            prefix.display()
        ))
        .arg(format!(
            "-DOPENSSL_CRYPTO_LIBRARY={}/lib/libcrypto.a",
            prefix.display()
        ))
        .arg("-DCURL_ZLIB=ON")
        .arg(format!("-DZLIB_INCLUDE_DIR={}/include", prefix.display()))
        .arg(format!("-DZLIB_LIBRARY={}/lib/libz.a", prefix.display()))
        .args([
            "-DHTTP_ONLY=OFF",
            "-DCURL_DISABLE_HTTP=OFF",
            "-DCURL_DISABLE_FTP=OFF",
            "-DCURL_CA_FALLBACK=ON",
            "-DCURL_USE_LIBPSL=OFF",
            "-DCURL_USE_LIBSSH2=OFF",
            "-DCURL_USE_LIBSSH=OFF",
            "-DCURL_USE_GSSAPI=OFF",
            "-DUSE_LIBIDN2=OFF",
            "-DCURL_BROTLI=OFF",
            "-DCURL_ZSTD=OFF",
            "-DUSE_NGHTTP2=OFF",
            "-DUSE_QUICHE=OFF",
            "-DCURL_DISABLE_LDAP=ON",
            "-DCURL_DISABLE_LDAPS=ON",
        ]);
    run(&mut cmake, "configure curl");

    let mut install_curl = Command::new("cmake");
    install_curl
        .arg("--build")
        .arg(&curl_build)
        .arg("--target")
        .arg("install")
        .arg("--parallel")
        .arg(jobs);
    run(&mut install_curl, "build and install curl");
}

fn target_settings(target: &str) -> (&'static str, &'static str, &'static str) {
    match target {
        "x86_64-unknown-linux-gnu" | "x86_64-unknown-linux-musl" => {
            ("linux-x86_64", "Linux", "x86_64")
        }
        "aarch64-unknown-linux-gnu" | "aarch64-unknown-linux-musl" => {
            ("linux-aarch64", "Linux", "aarch64")
        }
        "x86_64-apple-darwin" => ("darwin64-x86_64-cc", "Darwin", "x86_64"),
        "aarch64-apple-darwin" => ("darwin64-arm64-cc", "Darwin", "arm64"),
        other => panic!("unsupported vendored network target: {}", other),
    }
}

fn fresh_dir(path: &Path) {
    if path.exists() {
        fs::remove_dir_all(path).expect("remove stale native build directory");
    }
    fs::create_dir_all(path).expect("create native build directory");
}

fn run(command: &mut Command, description: &str) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("failed to execute {}: {}", description, error));
    assert!(status.success(), "{} failed with {}", description, status);
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}
