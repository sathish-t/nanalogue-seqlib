// Copyright 2014 Christopher Schröder, Johannes Köster.
// Licensed under the MIT license (http://opensource.org/licenses/MIT)
// This file may not be copied, modified, or distributed
// except according to those terms.

//! Module with utility code.

use crate::errors::{Error, Result};
use std::ffi;
use std::path::Path;

/// Copies data from `src` to `dst`
///
/// Panics if the length of `dst` is less than the length of `src`.
#[inline]
pub fn copy_memory(src: &[u8], dst: &mut [u8]) {
    if src.is_empty() {
        return;
    }
    let len_src = src.len();
    assert!(
        len_src <= 256 * 1024 * 1024,
        "cannot copy a location larger than 256 MiB"
    );
    assert!(
        dst.len() >= len_src,
        "dst len {} < src len {}",
        dst.len(),
        src.len()
    );
    dst[..len_src].copy_from_slice(src);
}

pub fn path_to_cstring<P: AsRef<Path>>(path: &P) -> Option<ffi::CString> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return None;
    }
    ffi::CString::new(path_bytes(path).ok()?).ok()
}

/// Convert a path into a byte-vector
pub fn path_as_bytes<'a, P: 'a + AsRef<Path>>(path: P, must_exist: bool) -> Result<Vec<u8>> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return if must_exist {
            Err(Error::FileNotFound {
                path: path.to_owned(),
            })
        } else {
            Err(Error::FileOpen {
                path: String::new(),
            })
        };
    }
    if path.exists() || !must_exist {
        path_bytes(path)
    } else {
        Err(Error::FileNotFound {
            path: path.to_owned(),
        })
    }
}

pub fn path_bytes(path: &Path) -> Result<Vec<u8>> {
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;

    #[cfg(unix)]
    let bytes = path.as_os_str().as_bytes().to_owned();
    #[cfg(not(unix))]
    let bytes = path
        .to_str()
        .ok_or(Error::NonUnicodePath)?
        .as_bytes()
        .to_owned();

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_paths() {
        assert_eq!(path_to_cstring(&""), None);
        assert_eq!(
            path_as_bytes("", true),
            Err(Error::FileNotFound {
                path: std::path::PathBuf::new()
            })
        );
        assert_eq!(
            path_as_bytes("", false),
            Err(Error::FileOpen {
                path: String::new()
            })
        );
    }

    #[cfg(unix)]
    #[test]
    fn preserves_non_unicode_path_bytes() {
        use std::os::unix::ffi::OsStringExt;

        let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(vec![b'\xFF']));
        assert_eq!(path_as_bytes(&path, false), Ok(vec![b'\xFF']));
        assert_eq!(path_to_cstring(&path).unwrap().as_bytes(), b"\xFF");
    }

    #[test]
    fn copy_memory_copies_only_the_source_prefix() {
        let src = [1, 2, 3];
        let mut dst = [0, 0, 0, 9, 9];

        copy_memory(&src, &mut dst);

        assert_eq!(dst, [1, 2, 3, 9, 9]);
    }

    #[test]
    fn copy_memory_accepts_an_empty_source() {
        let mut dst = [9, 9];

        copy_memory(&[], &mut dst);

        assert_eq!(dst, [9, 9]);
    }

    #[test]
    #[should_panic(expected = "dst len 2 < src len 3")]
    fn copy_memory_rejects_a_too_short_destination() {
        let mut dst = [0, 0];

        copy_memory(&[1, 2, 3], &mut dst);
    }
}
