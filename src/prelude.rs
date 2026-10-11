use std::{
    ffi::{CStr, CString, c_char},
    marker::PhantomData,
    mem::MaybeUninit,
};

// lol fight me if you hate this
use crate::sys::{
    CXCursorKind, CXError_ASTReadError, CXError_Crashed, CXError_Failure, CXError_InvalidArguments,
    CXErrorCode, autoc_Completion, autoc_Completion_Chunk, autoc_Completion_Chunk_CurrentParam,
    autoc_Completion_Chunk_Informative, autoc_Completion_Chunk_Optional,
    autoc_Completion_Chunk_Placeholder, autoc_Completion_Chunk_Punct,
    autoc_Completion_Chunk_ResultType, autoc_Completion_Chunk_Text,
    autoc_Completion_Chunk_TypedText, autoc_Completion_Chunks, autoc_Completions,
    autoc_Completions_dispose, autoc_Completions_next, autoc_Doc, autoc_Doc_code_complete,
    autoc_Doc_dispose, autoc_Doc_new, autoc_Doc_reparse,
};

pub struct Doc {
    inner: autoc_Doc,
    _file: CString,
}

unsafe impl Send for Doc {}

impl Doc {
    pub fn new(path: &str, source: &str, args: &[&str]) -> Result<Self, DocError> {
        let path = CString::new(path)?;

        // SAFETY: argv is short-lived, no need to preserve
        //         it dies after autoc_Doc_new()
        let cargs: Vec<CString> = args
            .iter()
            .map(|arg| CString::new(*arg))
            .collect::<Result<Vec<_>, std::ffi::NulError>>()?;
        let argv: Vec<*const c_char> = cargs.iter().map(|a| a.as_ptr()).collect();

        let doc = unsafe {
            autoc_Doc_new(
                path.as_ptr() as _,
                source.len() as _,
                source.as_ptr() as _,
                args.len() as _,
                argv.as_ptr() as _,
            )
        };
        if doc.translation_unit.is_null() {
            unsafe {
                autoc_Doc_dispose(doc);
            }
            return Err(DocError::ParseFailed);
        }

        Ok(Self {
            inner: doc,
            _file: path,
        })
    }

    pub fn update(&mut self, source: &str) -> Result<(), DocUpdateError> {
        self.inner.buf = source.as_ptr() as _;
        self.inner.len = source.len() as _;

        unsafe {
            match autoc_Doc_reparse(&mut self.inner) {
                0 => Ok(()),
                code => Err(code.into()),
            }
        }
    }

    pub fn complete(&mut self, line: u32, col: u32) -> Completions<'_> {
        let raw = unsafe { autoc_Doc_code_complete(&mut self.inner, line, col) };
        Completions {
            inner: raw,
            _doc: PhantomData,
        }
    }
}

impl Drop for Doc {
    fn drop(&mut self) {
        unsafe { autoc_Doc_dispose(self.inner) }
    }
}

pub struct Completions<'doc> {
    inner: autoc_Completions,
    _doc: PhantomData<&'doc mut Doc>,
}

impl Iterator for Completions<'_> {
    type Item = Completion;

    fn next(&mut self) -> Option<Self::Item> {
        let mut slot = MaybeUninit::<autoc_Completion>::uninit();
        let maybe_res = unsafe { autoc_Completions_next(&mut self.inner, slot.as_mut_ptr()) };
        if maybe_res != 0 {
            return None;
        }

        let compl = unsafe { slot.assume_init() };

        let out = Completion {
            chunks: unsafe { convert(&compl.chunks) },
            priority: compl.priority,
            cursor_kind: compl.cursor_kind,
            brief: if compl.brief.is_null() {
                None
            } else {
                Some(unsafe { cstr_to_string(compl.brief) })
            },
        };

        Some(out)
    }
}

impl Drop for Completions<'_> {
    fn drop(&mut self) {
        unsafe {
            autoc_Completions_dispose(self.inner);
        }
    }
}

unsafe fn convert(cs: &autoc_Completion_Chunks) -> Vec<CompletionChunk> {
    let items: &[autoc_Completion_Chunk] = if cs.items.is_null() {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(cs.items, cs.count) }
    };
    items
        .iter()
        .filter_map(|c| {
            #[allow(non_upper_case_globals)]
            Some(match c.kind {
                autoc_Completion_Chunk_TypedText => {
                    CompletionChunk::TypedText(unsafe { cstr_to_string(c.as_.text) })
                }
                autoc_Completion_Chunk_Text => {
                    CompletionChunk::Text(unsafe { cstr_to_string(c.as_.text) })
                }
                autoc_Completion_Chunk_ResultType => {
                    CompletionChunk::ResultType(unsafe { cstr_to_string(c.as_.text) })
                }
                autoc_Completion_Chunk_Placeholder => {
                    CompletionChunk::Placeholder(unsafe { cstr_to_string(c.as_.text) })
                }
                autoc_Completion_Chunk_CurrentParam => {
                    CompletionChunk::CurrentParam(unsafe { cstr_to_string(c.as_.text) })
                }
                autoc_Completion_Chunk_Informative => {
                    CompletionChunk::Informative(unsafe { cstr_to_string(c.as_.text) })
                }
                autoc_Completion_Chunk_Punct => {
                    CompletionChunk::Punct(unsafe { cstr_to_string(c.as_.text) })
                }
                autoc_Completion_Chunk_Optional => {
                    CompletionChunk::Optional(unsafe { convert(&c.as_.optional) })
                }
                _ => return None,
            })
        })
        .collect()
}

unsafe fn cstr_to_string(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "wasm",
    derive(serde::Serialize),
    serde(tag = "type", content = "data", rename_all = "lowercase")
)]
pub enum CompletionChunk {
    TypedText(String),
    Text(String),
    ResultType(String),
    Placeholder(String),
    CurrentParam(String),
    Informative(String),
    Punct(String),
    Optional(Vec<CompletionChunk>),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
pub struct Completion {
    pub chunks: Vec<CompletionChunk>,
    pub priority: u32,
    pub cursor_kind: CXCursorKind,
    pub brief: Option<String>,
}

#[derive(Debug)]
#[repr(i8)]
pub enum DocUpdateError {
    AstReadError = -1,
    Crashed = -2,
    Failure = -3,
    InvalidArguments = -4,
}

impl From<CXErrorCode> for DocUpdateError {
    fn from(value: CXErrorCode) -> Self {
        #[allow(non_upper_case_globals)]
        match value {
            CXError_Failure => Self::Failure,
            CXError_Crashed => Self::Crashed,
            CXError_InvalidArguments => Self::InvalidArguments,
            CXError_ASTReadError => Self::AstReadError,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DocError {
    #[error(transparent)]
    NulInStr(#[from] std::ffi::NulError),

    #[error("failed to parse")]
    ParseFailed,
}
