use std::fs;
use std::process::Command;

fn compile_misuse(source: &str) -> String {
    let crate_dir = tempfile::tempdir().expect("create temporary crate");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let manifest = format!(
        "[package]\nname = \"raw-bam-mutation\"\nversion = \"0.0.0\"\nedition = \"2018\"\n\n[dependencies]\nrust-htslib = {{ path = {manifest_dir:?}, default-features = false }}\n"
    );

    fs::write(crate_dir.path().join("Cargo.toml"), manifest).expect("write manifest");
    fs::copy(
        std::path::Path::new(manifest_dir).join("Cargo.lock"),
        crate_dir.path().join("Cargo.lock"),
    )
    .expect("copy lockfile");
    fs::create_dir(crate_dir.path().join("src")).expect("create source directory");
    fs::write(crate_dir.path().join("src/main.rs"), source).expect("write misuse example");

    let output = Command::new(env!("CARGO"))
        .arg("check")
        .arg("--offline")
        .current_dir(crate_dir.path())
        .env(
            "CARGO_TARGET_DIR",
            std::path::Path::new(manifest_dir).join("target/compile-fail"),
        )
        .output()
        .expect("run cargo check for misuse example");

    assert!(!output.status.success(), "safe raw-state mutation compiled");

    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn raw_bam_mutation_requires_unsafe_code() {
    let record_field = compile_misuse(include_str!("../test/compile-fail/raw_bam_mutation.rs"));
    assert!(
        record_field.contains("error[E0616]"),
        "Record::inner misuse failed for an unexpected reason: {}",
        record_field
    );

    let record_mut = compile_misuse(include_str!("../test/compile-fail/record_inner_mut.rs"));
    assert!(
        record_mut.contains("error[E0133]"),
        "Record::inner_mut misuse failed for an unexpected reason: {}",
        record_mut
    );

    let header_mut = compile_misuse(include_str!("../test/compile-fail/header_inner_mut.rs"));
    assert!(
        header_mut.contains("error[E0133]"),
        "HeaderView::inner_mut misuse failed for an unexpected reason: {}",
        header_mut
    );
}
