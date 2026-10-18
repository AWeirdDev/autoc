use std::{marker::PhantomData, ptr::NonNull};

use crate::{cxstring::OwnedCXString, sys::*};

pub struct CodeCompletionResults<'tu> {
    _phantom: PhantomData<&'tu ()>,
    inner: NonNull<CXCodeCompleteResults>,
}

impl<'tu> CodeCompletionResults<'tu> {
    #[inline]
    pub(crate) fn from_raw(ptr: *mut CXCodeCompleteResults) -> Option<Self> {
        Some(Self {
            _phantom: PhantomData,
            inner: NonNull::new(ptr)?,
        })
    }

    #[inline]
    fn get_raw_results(&self) -> *mut CXCompletionResult {
        unsafe { self.inner.as_ref() }.Results
    }

    #[inline]
    pub fn len(&self) -> std::ffi::c_uint {
        unsafe { self.inner.as_ref() }.NumResults
    }

    /// Sort the results in case-insensitive alphabetical.
    pub fn sort(&mut self) {
        unsafe {
            clang_sortCodeCompletionResults(self.get_raw_results(), self.len());
        }
    }

    pub fn iter<'res>(&'res self) -> CodeCompletionResultsIter<'res, 'tu> {
        CodeCompletionResultsIter {
            results: self,
            count: self.len(),
            index: 0,
        }
    }
}

pub struct CodeCompletionResultsIter<'res, 'tu> {
    results: &'res CodeCompletionResults<'tu>,
    count: std::ffi::c_uint,
    index: std::ffi::c_uint,
}

impl<'res, 'tu> Iterator for CodeCompletionResultsIter<'res, 'tu> {
    type Item = CodeCompletion<'res>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.count {
            let result = unsafe { self.results.get_raw_results().add(self.index as usize) };
            self.index += 1;

            let completion_string = unsafe { &*result }.CompletionString;

            if unsafe { clang_getCompletionAvailability(completion_string) }
                != CXAvailability_Available
            {
                continue;
            }

            let brief =
                OwnedCXString::from(unsafe { clang_getCompletionBriefComment(completion_string) });
            let priority = unsafe { clang_getCompletionPriority(completion_string) };

            return Some(CodeCompletion {
                chunks: completion_chunks(completion_string),
                priority,
                cursor_kind: unsafe { &*result }.CursorKind,
                brief,
            });
        }

        None
    }
}

pub type CodeCompletionChunks<'res> = Vec<CodeCompletionChunk<'res>>;

#[cfg_attr(feature = "wasm", derive(serde::Serialize), serde(transparent))]
pub struct ThinRef<'a, T> {
    _phantom: PhantomData<&'a ()>,
    inner: T,
}

impl<'a, T> ThinRef<'a, T> {
    #[inline]
    const fn new(value: T) -> Self {
        Self {
            _phantom: PhantomData,
            inner: value,
        }
    }
}

impl<'a, T> core::ops::Deref for ThinRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> core::ops::DerefMut for ThinRef<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg_attr(
    feature = "wasm",
    derive(serde::Serialize),
    serde(tag = "type", content = "data", rename_all = "camelCase")
)]
pub enum CodeCompletionChunk<'res> {
    Optional(CodeCompletionChunks<'res>),
    TypedText(ThinRef<'res, OwnedCXString>),
    Text(ThinRef<'res, OwnedCXString>),
    Placeholder(ThinRef<'res, OwnedCXString>),
    Informative(ThinRef<'res, OwnedCXString>),
    CurrentParameter(ThinRef<'res, OwnedCXString>),
    LeftParen(ThinRef<'res, OwnedCXString>),
    RightParen(ThinRef<'res, OwnedCXString>),
    LeftBracket(ThinRef<'res, OwnedCXString>),
    RightBracket(ThinRef<'res, OwnedCXString>),
    LeftBrace(ThinRef<'res, OwnedCXString>),
    RightBrace(ThinRef<'res, OwnedCXString>),
    LeftAngle(ThinRef<'res, OwnedCXString>),
    RightAngle(ThinRef<'res, OwnedCXString>),
    Comma(ThinRef<'res, OwnedCXString>),
    ResultType(ThinRef<'res, OwnedCXString>),
    Colon(ThinRef<'res, OwnedCXString>),
    SemiColon(ThinRef<'res, OwnedCXString>),
    Equal(ThinRef<'res, OwnedCXString>),
    HorizontalSpace(ThinRef<'res, OwnedCXString>),
    VerticalSpace(ThinRef<'res, OwnedCXString>),
}

#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
pub struct CodeCompletion<'res> {
    /// Completion chunks.
    pub chunks: CodeCompletionChunks<'res>,

    /// Determine the priority of this code completion.
    ///
    /// The priority of a code completion indicates how likely it is
    /// that this particular completion is the completion that the user
    /// will select. The priority is selected by various internal
    /// heuristics.
    pub priority: std::ffi::c_uint,

    /// Describes the kind of entity that a cursor refers to.
    pub cursor_kind: CXCursorKind,

    /// The brief documentation comment attached to the declaration that
    /// corresponds to the given completion string.
    pub brief: OwnedCXString,
}

fn completion_chunks<'res>(completion_str: CXCompletionString) -> CodeCompletionChunks<'res> {
    let n_completion_chunks = unsafe { clang_getNumCompletionChunks(completion_str) };
    let mut chunks = Vec::with_capacity(n_completion_chunks as usize);

    for chunk_index in 0..n_completion_chunks {
        let chunk = completion_chunk(completion_str, chunk_index);
        chunks.push(chunk);
    }

    chunks
}

fn completion_chunk<'res>(
    completion_str: CXCompletionString,
    chunk_index: std::ffi::c_uint,
) -> CodeCompletionChunk<'res> {
    let chunk_kind = unsafe { clang_getCompletionChunkKind(completion_str, chunk_index) };

    #[allow(nonstandard_style)]
    match chunk_kind {
        CXCompletionChunk_Optional => {
            let complstr =
                unsafe { clang_getCompletionChunkCompletionString(completion_str, chunk_index) };
            completion_chunk(complstr, chunk_index)
        }
        CXCompletionChunk_TypedText => {
            CodeCompletionChunk::TypedText(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_Text => {
            CodeCompletionChunk::Text(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_Placeholder => {
            CodeCompletionChunk::Placeholder(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_Informative => {
            CodeCompletionChunk::Informative(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_CurrentParameter => {
            CodeCompletionChunk::CurrentParameter(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_LeftParen => {
            CodeCompletionChunk::LeftParen(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_RightParen => {
            CodeCompletionChunk::RightParen(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_LeftBracket => {
            CodeCompletionChunk::LeftBracket(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_RightBracket => {
            CodeCompletionChunk::RightBracket(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_LeftBrace => {
            CodeCompletionChunk::LeftBrace(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_RightBrace => {
            CodeCompletionChunk::RightBrace(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_LeftAngle => {
            CodeCompletionChunk::LeftAngle(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_RightAngle => {
            CodeCompletionChunk::RightAngle(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_Comma => {
            CodeCompletionChunk::Comma(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_ResultType => {
            CodeCompletionChunk::ResultType(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_Colon => {
            CodeCompletionChunk::Colon(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_SemiColon => {
            CodeCompletionChunk::SemiColon(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_Equal => {
            CodeCompletionChunk::Equal(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_HorizontalSpace => {
            CodeCompletionChunk::HorizontalSpace(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        CXCompletionChunk_VerticalSpace => {
            CodeCompletionChunk::VerticalSpace(ThinRef::new(OwnedCXString::from(unsafe {
                clang_getCompletionChunkText(completion_str, chunk_index)
            })))
        }
        _ => unreachable!("unknown chunk code"),
    }
}
