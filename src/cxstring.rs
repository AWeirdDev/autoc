use std::ffi::CStr;

use crate::sys::{CXString, clang_disposeString, clang_getCString};

/// A [`CXString`] with ownership rules applied.
#[repr(transparent)]
pub struct OwnedCXString {
    inner: CXString,
}

impl OwnedCXString {
    /// Retrieve the associated character data in a [`CStr`] reference.
    #[inline]
    pub fn as_cstr(&self) -> &CStr {
        let cstr = unsafe { clang_getCString(self.inner) };
        unsafe { CStr::from_ptr(cstr) }
    }

    /// Converts to a `&str` if it contains valid UTF-8, which is not
    /// a guarantee from libclang.
    #[inline]
    pub fn to_str(&self) -> Result<&str, std::str::Utf8Error> {
        self.as_cstr().to_str()
    }

    /// Get the raw byte data without the NUL byte.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        let cstr = self.as_cstr();

        // NOTE: this is so sus... like why does everyone love fat pointers
        //       is that obesity???? omgomgmogm hey it's me it's verity
        unsafe { core::slice::from_raw_parts(cstr.as_ptr() as _, cstr.count_bytes()) }
    }
}

impl From<CXString> for OwnedCXString {
    #[inline]
    fn from(value: CXString) -> Self {
        Self { inner: value }
    }
}

impl core::fmt::Display for OwnedCXString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_cstr().to_string_lossy())
    }
}

impl Drop for OwnedCXString {
    fn drop(&mut self) {
        unsafe { clang_disposeString(self.inner) };
    }
}

#[cfg(feature = "wasm")]
impl serde::Serialize for OwnedCXString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
