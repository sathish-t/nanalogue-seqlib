// Copyright 2014 Christopher Schröder, Johannes Köster.
// Licensed under the MIT license (http://opensource.org/licenses/MIT)
// This file may not be copied, modified, or distributed
// except according to those terms.

//! Module with utility code.

use crate::errors::{Error, Result};
use std::ffi;
use std::path::Path;
use std::ptr;

/// Copies data from `src` to `dst`
/// TODO remove once stable in standard library.
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
    // `dst` is unaliasable, so we know statically it doesn't overlap
    // with `src`.
    unsafe {
        ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr(), len_src);
    }
}

pub fn path_to_cstring<P: AsRef<Path>>(path: &P) -> Option<ffi::CString> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return None;
    }
    path.to_str().and_then(|p| ffi::CString::new(p).ok())
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
        Ok(path
            .to_str()
            .ok_or(Error::NonUnicodePath)?
            .as_bytes()
            .to_owned())
    } else {
        Err(Error::FileNotFound {
            path: path.to_owned(),
        })
    }
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
}
