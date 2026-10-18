use std::{
    ffi::CString,
    mem::{self, MaybeUninit},
};

use crate::{completions::CodeCompletionResults, diagnostics::DiagnosticSet, sys::*};

pub struct Doc<B: AsRef<[u8]>> {
    /// The underlying translation unit.
    tu: CXTranslationUnit,

    /// An index which consists of a set of translation units
    /// which can be used to link together into an executable
    /// or library.
    index: CXIndex,

    /// The path of the document. This is only for diagnostic
    /// display reasons.
    path: CString,

    /// The underlying source buffer.
    buf: B,
}

impl<B: AsRef<[u8]>> Doc<B> {
    /// Creates a new document.
    ///
    /// The `source` buffer should be kept alive while this `Doc<'buf>`
    /// is present.
    ///
    /// There should be no NUL byte (`\0`) in `path` or items of `args`.
    pub fn new(path: &str, source: B, args: &[&str]) -> Result<Self, DocCreateError> {
        let index = unsafe {
            clang_createIndex(
                /* excludeDeclarationsFromPCH */ 0, /* displayDiagnostics */ 0,
            )
        };

        let args = args
            .iter()
            .map(|arg| CString::new(*arg))
            .collect::<Result<Vec<_>, std::ffi::NulError>>()?;
        let raw_args = {
            let mut container = Vec::with_capacity(args.len());
            container.extend(args.iter().map(|cstr| cstr.as_ptr()));
            container
        };

        let path = CString::new(path)?;

        let unsaved = CXUnsavedFile {
            Filename: path.as_ptr() as _,
            Contents: source.as_ref().as_ptr() as _,
            Length: source.as_ref().len() as _,
        };
        let mut raw_unsaved = [unsaved];

        let mut tu = MaybeUninit::uninit();
        let result = unsafe {
            clang_parseTranslationUnit2(
                index,
                path.as_ptr() as _,
                raw_args.as_ptr() as _,
                raw_args.len() as _,
                raw_unsaved.as_mut_ptr(),
                1,
                CXTranslationUnit_PrecompiledPreamble
                    | CXTranslationUnit_CacheCompletionResults
                    | CXTranslationUnit_IncludeBriefCommentsInCodeCompletion,
                tu.as_mut_ptr(),
            )
        };

        #[allow(nonstandard_style)]
        match result {
            CXError_Success => (),
            _ => return Err(DocCreateError::CXError(CXError::from(result))),
        };

        let tu = unsafe { tu.assume_init() };

        Ok(Self {
            tu,
            index,
            path,
            buf: source,
        })
    }

    /// Retrieve the underlying diagnostics.
    ///
    /// Diagnostics may be generated when parsing. For instance, right
    /// after `Doc::new`, you can check if there are any diagnostics such
    /// as parse failures.
    #[inline]
    pub fn get_diagnostics<'tu>(&'tu self) -> DiagnosticSet<'tu> {
        DiagnosticSet::from_translation_unit(self.tu)
    }

    /// Replace the underlying buffer.
    ///
    /// This directly replaces the current one with the new one (`buf`).
    pub fn replace_buf(&mut self, buf: B) {
        self.buf = buf;
    }

    #[inline]
    fn get_unsaved_files(&self) -> [CXUnsavedFile; 1] {
        [CXUnsavedFile {
            Filename: self.path.as_ptr() as _,
            Contents: self.buf.as_ref().as_ptr() as _,
            Length: self.buf.as_ref().len() as _,
        }]
    }

    /// Edit the underlying source buffer.
    pub fn edit<R>(&mut self, f: impl FnOnce(&mut B) -> R) -> Result<R, CXError> {
        let out = f(&mut self.buf);
        self.reparse()?;
        Ok(out)
    }

    /// Reparse the source that produced this translation unit.
    pub fn reparse(&self) -> Result<(), CXError> {
        let mut raw_unsaved = self.get_unsaved_files();
        let result = unsafe {
            clang_reparseTranslationUnit(
                self.tu,
                1,
                raw_unsaved.as_mut_ptr(),
                clang_defaultReparseOptions(self.tu),
            )
        };

        #[allow(nonstandard_style)]
        match result as _ {
            CXError_Success => Ok(()),
            code => Err(CXError::from(code)),
        }
    }

    /// Perform code completion at a given location in a translation unit.
    pub fn code_complete<'tu>(
        &'tu self,
        line: std::ffi::c_uint,
        column: std::ffi::c_uint,
    ) -> Option<CodeCompletionResults<'tu>> {
        let mut raw_unsaved = self.get_unsaved_files();
        let results = unsafe {
            clang_codeCompleteAt(
                self.tu,
                self.path.as_ptr() as _,
                line,
                column,
                raw_unsaved.as_mut_ptr(),
                1,
                clang_defaultCodeCompleteOptions() | CXCodeComplete_IncludeBriefComments,
            )
        };
        CodeCompletionResults::from_raw(results)
    }
}

impl<B: AsRef<[u8]>> Drop for Doc<B> {
    fn drop(&mut self) {
        unsafe {
            clang_disposeTranslationUnit(self.tu);
            clang_disposeIndex(self.index);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DocCreateError {
    #[error(transparent)]
    NulError(#[from] std::ffi::NulError),

    #[error("libclang error: {0:?}")]
    CXError(CXError),
}

/// Error codes returned by libclang routines.
///
/// The error codes start from `1`.
#[repr(u8)]
#[derive(Debug)]
pub enum CXError {
    #[doc = " A generic error code, no further details are available.\n\n"]
    #[doc = "Errors of this kind can get their own specific error codes in future\n libclang versions."]
    Failure = 1,

    #[doc = " libclang crashed while performing the requested operation."]
    Crashed = 2,

    #[doc = " The function detected that the arguments violate the function\n contract."]
    InvalidArguments = 3,

    #[doc = " An AST deserialization error has occurred."]
    ASTReadError = 4,
}

impl From<CXErrorCode> for CXError {
    fn from(value: CXErrorCode) -> Self {
        #[allow(nonstandard_style)]
        match value {
            CXError_Success => unreachable!("error code 'success'"),
            1..=4 => {
                // SAFETY: repr(u8)
                unsafe { mem::transmute::<u8, Self>(value as u8) }
            }
            _ => unreachable!("unknown CXErrorCode"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    const SOURCE: &str = r#"struct Point { int x; int y; void move69(int dx, int dy = 0); };
int main() { Point p; p.mo
}"#;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error>> {
        let buf = String::from(SOURCE);
        let doc = Doc::new("main.c", buf, &[])?;
        let completions = doc.code_complete(2, 26);
        assert!(matches!(completions, Some(_)));
        Ok(())
    }
}
