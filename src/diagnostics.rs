use std::marker::PhantomData;

use crate::{cxstring::OwnedCXString, sys::*};

/// A group of diagnostics.
///
/// It should live as long as the translation unit, thus
/// the lifetime `'tu`.
pub struct DiagnosticSet<'tu> {
    _phantom: PhantomData<&'tu ()>,
    tu: CXTranslationUnit,
    inner: CXDiagnosticSet,
}

impl<'tu> DiagnosticSet<'tu> {
    #[inline]
    pub(crate) fn from_translation_unit(tu: CXTranslationUnit) -> Self {
        Self {
            _phantom: PhantomData,
            tu,
            inner: unsafe { clang_getDiagnosticSetFromTU(tu) },
        }
    }

    #[inline]
    pub fn iter<'set>(&'set mut self) -> DiagnosticsIter<'set, 'tu> {
        let n_diagnostics = self.len();
        DiagnosticsIter {
            count: n_diagnostics,
            index: 0,
            set: self,
        }
    }

    #[inline]
    pub unsafe fn get_unchecked<'set>(&'set self, index: std::ffi::c_uint) -> Diagnostic<'set> {
        let diagnostic = unsafe { clang_getDiagnosticInSet(self.inner, index) };
        Diagnostic::from_raw(diagnostic)
    }

    #[inline]
    pub fn get<'set>(&'set self, index: std::ffi::c_uint) -> Option<Diagnostic<'set>> {
        if index >= self.len() {
            None
        } else {
            let diagnostic = unsafe { clang_getDiagnosticInSet(self.inner, index) };
            Some(Diagnostic::from_raw(diagnostic))
        }
    }

    #[inline]
    pub fn len(&self) -> std::ffi::c_uint {
        unsafe { clang_getNumDiagnosticsInSet(self.inner) }
    }
}

impl Drop for DiagnosticSet<'_> {
    fn drop(&mut self) {
        unsafe { clang_disposeDiagnosticSet(self.inner) };
    }
}

pub struct DiagnosticsIter<'set, 'tu> {
    count: std::ffi::c_uint,
    index: std::ffi::c_uint,
    set: &'set DiagnosticSet<'tu>,
}

impl<'set, 'tu> Iterator for DiagnosticsIter<'set, 'tu> {
    type Item = Diagnostic<'set>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }

        let diagnostic = unsafe { clang_getDiagnostic(self.set.tu, self.index) };
        self.index += 1;

        Some(Diagnostic::from_raw(diagnostic))
    }
}

/// A diagnostic should live beside as long as a [`DiagnosticSet`].
#[repr(transparent)]
pub struct Diagnostic<'set> {
    _phantom: PhantomData<&'set ()>,
    inner: CXDiagnostic,
}

impl Diagnostic<'_> {
    #[inline]
    fn from_raw(diagnostic: CXDiagnostic) -> Self {
        Self {
            _phantom: PhantomData,
            inner: diagnostic,
        }
    }

    /// Retrieve the category number for this diagnostic.
    ///
    /// Diagnostics can be categorized into groups along with other,
    /// related diagnostics (e.g., diagnostics under the same warning flag).
    ///
    /// A `None` is returned if it's uncategorized.
    pub fn get_category(&self) -> Option<std::ffi::c_uint> {
        let maybe_category = unsafe { clang_getDiagnosticCategory(self.inner) };
        match maybe_category {
            0 => None,
            cat => Some(cat),
        }
    }

    /// Retrieve the diagnostic category text for this diagnostic.
    #[inline]
    pub fn get_category_text(&self) -> OwnedCXString {
        OwnedCXString::from(unsafe { clang_getDiagnosticCategoryText(self.inner) })
    }
}

impl core::fmt::Display for Diagnostic<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted = OwnedCXString::from(unsafe {
            clang_formatDiagnostic(self.inner, clang_defaultDiagnosticDisplayOptions())
        });
        formatted.fmt(f)
    }
}

impl Drop for Diagnostic<'_> {
    fn drop(&mut self) {
        unsafe { clang_disposeDiagnostic(self.inner) };
    }
}
