//! FASTA index construction.

use crate::errors::Error;
use crate::htslib;

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
    let os_path = std::ffi::CString::new(path.display().to_string())?;
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
