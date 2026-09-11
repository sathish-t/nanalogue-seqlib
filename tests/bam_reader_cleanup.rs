#[cfg(target_os = "linux")]
#[test]
fn indexed_reader_releases_handles_when_index_loading_fails() {
    use rust_htslib::bam::IndexedReader;

    let descriptors_before = std::fs::read_dir("/proc/self/fd").unwrap().count();

    for _ in 0..10 {
        assert!(IndexedReader::from_path_and_index("test/test.bam", "Cargo.toml").is_err());
    }

    assert_eq!(
        std::fs::read_dir("/proc/self/fd").unwrap().count(),
        descriptors_before
    );
}
