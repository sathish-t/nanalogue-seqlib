//! Build vendored curl against the directly compiled TLS stack.

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::native_target::{Os, Target};

/// Builds static curl into `out/native`, using its existing OpenSSL archives.
///
/// `compiler` and `archiver` are the project-created wrappers around the
/// pinned Zig `cc` and `ar`; CMake never discovers a host compiler or OpenSSL.
pub fn build(out: &Path, target: Target, compiler: &Path, archiver: &Path) {
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
    let curl_build = out.join("curl-build");
    let ranlib_wrapper = out.join("zig-ranlib.sh");
    let jobs = env::var("NUM_JOBS").unwrap_or_else(|_| "1".into());
    let (system, processor) = target.cmake();

    fs::create_dir_all(&prefix).expect("create native installation prefix");

    // Keep curl's archive index operation on the supplied Zig archiver.
    fs::write(
        &ranlib_wrapper,
        format!("#!/bin/sh\nexec {} s \"$@\"\n", shell_quote(archiver)),
    )
    .expect("write Zig ranlib wrapper");
    fs::set_permissions(&ranlib_wrapper, fs::Permissions::from_mode(0o755))
        .expect("make Zig ranlib wrapper executable");
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
        // CMake's FindOpenSSL otherwise probes pkg-config independently of curl.
        // Keep FindPkgConfig's macros available for older CMake versions.
        .arg("-DPKG_CONFIG_EXECUTABLE=/bin/false")
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
            "-DCURL_DISABLE_DICT=ON",
            "-DCURL_DISABLE_DOH=ON",
            "-DCURL_DISABLE_FILE=ON",
            "-DCURL_DISABLE_GOPHER=ON",
            "-DCURL_DISABLE_IMAP=ON",
            "-DCURL_DISABLE_IPFS=ON",
            "-DCURL_DISABLE_MQTT=ON",
            "-DCURL_DISABLE_POP3=ON",
            "-DCURL_DISABLE_RTSP=ON",
            "-DCURL_DISABLE_SMTP=ON",
            "-DCURL_DISABLE_TELNET=ON",
            "-DCURL_DISABLE_TFTP=ON",
            "-DCURL_DISABLE_WEBSOCKETS=ON",
            "-DCURL_ENABLE_SMB=OFF",
            "-DCURL_CA_FALLBACK=ON",
            "-DCURL_USE_LIBPSL=OFF",
            "-DCURL_USE_LIBSSH2=OFF",
            "-DCURL_USE_LIBSSH=OFF",
            "-DCURL_USE_GSSAPI=OFF",
            "-DUSE_LIBIDN2=OFF",
            "-DCURL_BROTLI=OFF",
            "-DCURL_ZSTD=OFF",
            "-DUSE_NGHTTP2=OFF",
            "-DUSE_NGTCP2=OFF",
            "-DUSE_QUICHE=OFF",
            "-DCURL_DISABLE_LDAP=ON",
            "-DCURL_DISABLE_LDAPS=ON",
        ]);
    if target.os == Os::Macos {
        cmake.arg("-DUSE_APPLE_SECTRUST=ON");
    }
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
