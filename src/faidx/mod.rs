//! FASTA index construction.

use crate::errors::Error;
use crate::htslib;
use crate::utils;

/// Build a faidx for input path.
///
/// # Errors
/// If indexing fails. Could be malformatted or file could not be accessible.
///
/// ```
/// use rust_htslib::faidx::build;
/// # let dir = tempfile::tempdir().unwrap();
/// # let path = dir.path().join("reference.fa");
/// # std::fs::write(&path, b">chr1\nACGT\n").unwrap();
/// build(&path).expect("Failed to build fasta index");
/// # assert_eq!(std::fs::read_to_string(path.with_extension("fa.fai")).unwrap(), "chr1\t4\t6\t4\t5\n");
/// ```
pub fn build(
    path: impl Into<std::path::PathBuf>,
) -> Result<(), std::boxed::Box<dyn std::error::Error>> {
    let path = path.into();
    if path.as_os_str().is_empty() {
        return Err(Box::new(Error::FaidxBuildFailed { path }));
    }
    let os_path = std::ffi::CString::new(utils::path_bytes(&path)?)?;
    let rc = unsafe { htslib::fai_build(os_path.as_ptr()) };
    if rc < 0 {
        Err(Error::FaidxBuildFailed { path })?
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.fa");
        let error = build(&path).unwrap_err();
        assert_eq!(
            error.downcast_ref::<Error>(),
            Some(&Error::FaidxBuildFailed { path })
        );
    }

    #[test]
    fn nul_in_path() {
        let error = build("invalid\0.fa").unwrap_err();
        assert!(error.downcast_ref::<std::ffi::NulError>().is_some());
    }

    #[cfg(unix)]
    #[test]
    fn builds_index_for_non_unicode_filename() {
        use std::os::unix::ffi::OsStringExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir
            .path()
            .join(std::ffi::OsString::from_vec(b"reference-\xFF.fa".to_vec()));
        std::fs::write(&path, b">chr1\nACGT\n").unwrap();

        build(&path).unwrap();

        assert_eq!(
            std::fs::read(path.with_extension("fa.fai")).unwrap(),
            b"chr1\t4\t6\t4\t5\n"
        );
    }

    #[test]
    fn empty_path() {
        let error = build("").unwrap_err();
        assert_eq!(
            error.downcast_ref::<Error>(),
            Some(&Error::FaidxBuildFailed {
                path: std::path::PathBuf::new()
            })
        );
    }
}
