//! Check the C/Rust ABI and that Cargo features really enable native codecs.
use rust_htslib::htslib;
use std::collections::BTreeMap;
#[cfg(feature = "lzma")]
use std::ffi::CStr;
use std::mem::{align_of, offset_of, size_of};

extern "C" {
    fn nanalogue_hts_abi_table(len: *mut usize) -> *const AbiEntry;
    #[cfg(feature = "lzma")]
    fn lzma_version_string() -> *const libc::c_char;

    #[cfg(feature = "curl")]
    fn OpenSSL_version_num() -> libc::c_ulong;
    #[cfg(feature = "curl")]
    fn OSSL_PROVIDER_load(
        libctx: *mut libc::c_void,
        name: *const libc::c_char,
    ) -> *mut libc::c_void;
    #[cfg(feature = "curl")]
    fn OSSL_PROVIDER_unload(provider: *mut libc::c_void) -> libc::c_int;
}

#[repr(C)]
struct AbiEntry {
    name: *const libc::c_char,
    value: u64,
}

/// Read the table that native/wrapper.c compiles from the real headers.
fn c_abi() -> BTreeMap<String, u64> {
    let mut len = 0;
    let entries = unsafe {
        let table = nanalogue_hts_abi_table(&mut len);
        std::slice::from_raw_parts(table, len)
    };
    entries
        .iter()
        .map(|e| {
            let name = unsafe { std::ffi::CStr::from_ptr(e.name) };
            (name.to_str().unwrap().to_owned(), e.value)
        })
        .collect()
}

macro_rules! size {
    ($t:ident) => {
        (concat!("size ", stringify!($t)), size_of::<htslib::$t>())
    };
    (enum $c:literal, $t:ident) => {
        (concat!("size enum ", $c), size_of::<htslib::$t>())
    };
}
macro_rules! align {
    ($t:ident) => {
        (concat!("align ", stringify!($t)), align_of::<htslib::$t>())
    };
}
macro_rules! offset {
    ($t:ident, $($f:ident).+) => {
        (
            concat!("offset ", stringify!($t), ".", stringify!($($f).+)),
            offset_of!(htslib::$t, $($f).+),
        )
    };
}
macro_rules! value {
    ($c:ident) => {
        (stringify!($c), htslib::$c as usize)
    };
    ($c:literal, $r:ident) => {
        ($c, htslib::$r as usize)
    };
}

/// Every concrete struct field and constant hand-declared in src/htslib.rs
/// must match the C compiler's view of the vendored headers.
#[test]
fn rust_bindings_match_zig_c_layouts() {
    let rust: Vec<(&str, usize)> = vec![
        size!(hts_pos_t),
        size!(enum "htsLogLevel", htsLogLevel),
        size!(enum "htsExactFormat", htsExactFormat),
        size!(enum "hts_fmt_option", hts_fmt_option),
        size!(enum "sam_fields", sam_fields),
        size!(htsFormat),
        align!(htsFormat),
        offset!(htsFormat, category),
        offset!(htsFormat, format),
        offset!(htsFormat, version),
        offset!(htsFormat, version.minor),
        offset!(htsFormat, compression),
        offset!(htsFormat, compression_level),
        offset!(htsFormat, specific),
        size!(htsThreadPool),
        align!(htsThreadPool),
        offset!(htsThreadPool, pool),
        offset!(htsThreadPool, qsize),
        size!(sam_hdr_t),
        align!(sam_hdr_t),
        offset!(sam_hdr_t, n_targets),
        offset!(sam_hdr_t, ignore_sam_err),
        offset!(sam_hdr_t, l_text),
        offset!(sam_hdr_t, target_len),
        offset!(sam_hdr_t, cigar_tab),
        offset!(sam_hdr_t, target_name),
        offset!(sam_hdr_t, text),
        offset!(sam_hdr_t, sdict),
        offset!(sam_hdr_t, hrecs),
        offset!(sam_hdr_t, ref_count),
        size!(bam1_core_t),
        align!(bam1_core_t),
        offset!(bam1_core_t, pos),
        offset!(bam1_core_t, tid),
        offset!(bam1_core_t, bin),
        offset!(bam1_core_t, qual),
        offset!(bam1_core_t, l_extranul),
        offset!(bam1_core_t, flag),
        offset!(bam1_core_t, l_qname),
        offset!(bam1_core_t, n_cigar),
        offset!(bam1_core_t, l_qseq),
        offset!(bam1_core_t, mtid),
        offset!(bam1_core_t, mpos),
        (
            "offset bam1_core_t.isize",
            offset_of!(htslib::bam1_core_t, isize_),
        ),
        size!(bam1_t),
        align!(bam1_t),
        offset!(bam1_t, core),
        offset!(bam1_t, id),
        offset!(bam1_t, data),
        offset!(bam1_t, l_data),
        offset!(bam1_t, m_data),
        value!("sam", htsExactFormat_sam),
        value!("bam", htsExactFormat_bam),
        value!("cram", htsExactFormat_cram),
        value!("CRAM_OPT_VERSION", hts_fmt_option_CRAM_OPT_VERSION),
        value!("CRAM_OPT_EMBED_REF", hts_fmt_option_CRAM_OPT_EMBED_REF),
        value!("CRAM_OPT_REFERENCE", hts_fmt_option_CRAM_OPT_REFERENCE),
        value!("CRAM_OPT_NO_REF", hts_fmt_option_CRAM_OPT_NO_REF),
        value!(
            "CRAM_OPT_REQUIRED_FIELDS",
            hts_fmt_option_CRAM_OPT_REQUIRED_FIELDS
        ),
        value!(
            "HTS_OPT_COMPRESSION_LEVEL",
            hts_fmt_option_HTS_OPT_COMPRESSION_LEVEL
        ),
        value!("SAM_FLAG", sam_fields_SAM_FLAG),
        value!("SAM_RNAME", sam_fields_SAM_RNAME),
        value!(HTS_FEATURE_PLUGINS),
        value!(HTS_FEATURE_LIBCURL),
        value!(HTS_FEATURE_S3),
        value!(HTS_FEATURE_GCS),
        value!(HTS_FEATURE_LIBDEFLATE),
        value!(HTS_FEATURE_LZMA),
        value!(HTS_FEATURE_BZIP2),
        value!(HTS_FEATURE_HTSCODECS),
        value!(BAM_FPAIRED),
        value!(BAM_FPROPER_PAIR),
        value!(BAM_FUNMAP),
        value!(BAM_FMUNMAP),
        value!(BAM_FREVERSE),
        value!(BAM_FMREVERSE),
        value!(BAM_FREAD1),
        value!(BAM_FREAD2),
        value!(BAM_FSECONDARY),
        value!(BAM_FQCFAIL),
        value!(BAM_FDUP),
        value!(BAM_FSUPPLEMENTARY),
    ];
    let count = rust.len();
    let rust: BTreeMap<String, u64> = rust
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value as u64))
        .collect();
    assert_eq!(rust.len(), count, "duplicate name in the Rust ABI list");
    // The immutable table is emitted by Zig from the actual compiled C headers.
    let c = c_abi();
    let names: std::collections::BTreeSet<&String> = rust.keys().chain(c.keys()).collect();
    let mismatches: Vec<String> = names
        .into_iter()
        .filter(|name| rust.get(*name) != c.get(*name))
        .map(|name| format!("{name}: Rust {:?}, C {:?}", rust.get(name), c.get(name)))
        .collect();
    assert!(
        mismatches.is_empty(),
        "src/htslib.rs disagrees with the HTSlib headers:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn cargo_features_match_compiled_htslib() {
    let features = unsafe { htslib::hts_features() };
    for (enabled, bit) in [
        (cfg!(feature = "bzip2"), htslib::HTS_FEATURE_BZIP2),
        (cfg!(feature = "lzma"), htslib::HTS_FEATURE_LZMA),
        (cfg!(feature = "libdeflate"), htslib::HTS_FEATURE_LIBDEFLATE),
        (cfg!(feature = "curl"), htslib::HTS_FEATURE_LIBCURL),
        (cfg!(feature = "s3"), htslib::HTS_FEATURE_S3),
        (cfg!(feature = "gcs"), htslib::HTS_FEATURE_GCS),
    ] {
        assert_eq!(features & bit != 0, enabled, "native feature {bit}");
    }
    assert_eq!(features & htslib::HTS_FEATURE_PLUGINS, 0);
    assert_ne!(features & htslib::HTS_FEATURE_HTSCODECS, 0);
}

#[cfg(feature = "lzma")]
#[test]
fn bundled_liblzma_is_5_8_4() {
    let version = unsafe { CStr::from_ptr(lzma_version_string()) };
    assert_eq!(version.to_bytes(), b"5.8.4");
}

#[cfg(feature = "curl")]
#[test]
fn vendored_openssl_has_expected_version_and_builtin_providers() {
    assert_eq!(unsafe { OpenSSL_version_num() }, 0x30600040);

    for name in [b"default\0".as_slice(), b"base\0".as_slice()] {
        let provider = unsafe {
            OSSL_PROVIDER_load(std::ptr::null_mut(), name.as_ptr().cast::<libc::c_char>())
        };
        assert!(!provider.is_null());
        assert_eq!(unsafe { OSSL_PROVIDER_unload(provider) }, 1);
    }
}

#[cfg(feature = "curl")]
#[test]
fn vendored_curl_is_8_22() {
    use std::ffi::{c_char, c_int, c_long, c_uint, CStr};

    #[repr(C)]
    struct CurlVersionInfo {
        age: c_int,
        version: *const c_char,
        version_num: c_uint,
        host: *const c_char,
        features: c_int,
        ssl_version: *const c_char,
        ssl_version_num: c_long,
        libz_version: *const c_char,
        protocols: *const *const c_char,
        ares: *const c_char,
        ares_num: c_int,
        libidn: *const c_char,
        iconv_ver_num: c_int,
        libssh_version: *const c_char,
        brotli_ver_num: c_uint,
        brotli_version: *const c_char,
        nghttp2_ver_num: c_uint,
        nghttp2_version: *const c_char,
        quic_version: *const c_char,
        cainfo: *const c_char,
        capath: *const c_char,
        zstd_ver_num: c_uint,
        zstd_version: *const c_char,
        hyper_version: *const c_char,
        gsasl_version: *const c_char,
        feature_names: *const *const c_char,
        rtmp_version: *const c_char,
    }

    extern "C" {
        fn curl_version() -> *const c_char;
        fn curl_version_info(age: c_int) -> *const CurlVersionInfo;
    }

    let version = unsafe { CStr::from_ptr(curl_version()) };
    assert!(
        version.to_bytes().starts_with(b"libcurl/8.22.0"),
        "unexpected curl version: {}",
        version.to_string_lossy()
    );

    let info = unsafe { &*curl_version_info(0) };
    let mut protocols = Vec::new();
    let mut current = info.protocols;
    unsafe {
        while !(*current).is_null() {
            protocols.push(CStr::from_ptr(*current).to_str().unwrap());
            current = current.add(1);
        }
    }
    assert_eq!(protocols, ["ftp", "ftps", "http", "https"]);

    let mut features = Vec::new();
    let mut current = info.feature_names;
    unsafe {
        while !(*current).is_null() {
            features.push(CStr::from_ptr(*current).to_str().unwrap());
            current = current.add(1);
        }
    }
    assert_eq!(
        features.contains(&"AppleSecTrust"),
        cfg!(target_os = "macos")
    );

    const SSL_AND_ZLIB: c_int = (1 << 2) | (1 << 3);
    const DISABLED_DEPENDENCIES: c_int =
        (1 << 5) | (1 << 8) | (1 << 10) | (1 << 16) | (1 << 20) | (1 << 23) | (1 << 25) | (1 << 26);
    assert_eq!(info.features & SSL_AND_ZLIB, SSL_AND_ZLIB);
    assert_eq!(info.features & DISABLED_DEPENDENCIES, 0);
}

#[cfg(feature = "curl")]
#[test]
fn vendored_curl_reads_bam_over_http() {
    use rust_htslib::bam::{Read, Reader};
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut request = BufReader::new(&stream);
        let mut line = String::new();
        request.read_line(&mut line).unwrap();
        assert!(line.starts_with("GET /test.bam HTTP/"), "{}", line);
        loop {
            line.clear();
            assert_ne!(request.read_line(&mut line).unwrap(), 0);
            if line == "\r\n" {
                break;
            }
        }
        let bam = include_bytes!("../test/test.bam");
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bam.len()
        )
        .unwrap();
        stream.write_all(bam).unwrap();
    });
    let url = url::Url::parse(&format!("http://{address}/test.bam")).unwrap();
    let mut bam = Reader::from_url(&url).unwrap();
    let names: Vec<_> = bam.records().map(|r| r.unwrap().qname().to_vec()).collect();
    assert_eq!(
        names,
        [
            b"I".to_vec(),
            b"II.14978392".to_vec(),
            b"III".to_vec(),
            b"IV".to_vec(),
            b"V".to_vec(),
            b"VI".to_vec()
        ]
    );
    server.join().unwrap();
}
