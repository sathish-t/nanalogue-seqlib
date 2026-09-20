//! Check the C/Rust ABI and that Cargo features really enable native codecs.
use rust_htslib::htslib;
use std::mem::{align_of, offset_of, size_of};

extern "C" {
    static nanalogue_hts_abi: [usize; 12];

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

#[test]
fn rust_bindings_match_zig_c_layouts() {
    let rust = [
        size_of::<htslib::bam1_t>(),
        align_of::<htslib::bam1_t>(),
        offset_of!(htslib::bam1_t, data),
        offset_of!(htslib::bam1_t, l_data),
        size_of::<htslib::bam1_core_t>(),
        offset_of!(htslib::bam1_core_t, pos),
        offset_of!(htslib::bam1_core_t, n_cigar),
        size_of::<htslib::sam_hdr_t>(),
        offset_of!(htslib::sam_hdr_t, target_len),
        size_of::<htslib::htsFile>(),
        offset_of!(htslib::htsFile, fp),
        size_of::<htslib::htsFormat>(),
    ];
    // The immutable array is emitted by Zig from the actual compiled C headers.
    assert_eq!(rust, unsafe { nanalogue_hts_abi });
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
