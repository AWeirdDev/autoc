use std::{
    cell::RefCell,
    ffi::{CStr, CString, c_char},
};

use autoc::prelude::*;

thread_local! { static DOCS_ARENA: RefCell<Arena<Doc<CStrBuffer<'static>>>> = RefCell::new(Arena::new()); }

#[repr(transparent)]
struct CStrBuffer<'a>(&'a CStr);

impl AsRef<[u8]> for CStrBuffer<'_> {
    fn as_ref(&self) -> &[u8] {
        self.0.to_bytes()
    }
}

impl core::ops::Deref for CStrBuffer<'_> {
    type Target = CStr;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

struct Arena<T> {
    items: Vec<Option<T>>,
    vacancies: Vec<usize>,
}

impl<T> Arena<T> {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            vacancies: Vec::new(),
        }
    }

    #[inline]
    fn push(&mut self, item: T) -> usize {
        if let Some(vacancy) = self.vacancies.pop() {
            self.items[vacancy].replace(item);
            vacancy
        } else {
            self.items.push(Some(item));
            self.items.len() - 1
        }
    }

    #[inline]
    fn take(&mut self, index: usize) -> Option<T> {
        self.items.get_mut(index).and_then(|item| item.take())
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index).and_then(|item| item.as_mut())
    }

    #[inline]
    fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index).and_then(|item| item.as_ref())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn doc_new(path: *mut c_char, src: *mut c_char, flags: *mut c_char) -> isize {
    let path = unsafe { CStr::from_ptr(path) }.to_string_lossy();
    let src = CStrBuffer(unsafe { CStr::from_ptr(src) });
    let flags = unsafe { CStr::from_ptr(flags) }.to_string_lossy();
    let args = flags.split("\n").collect::<Vec<_>>();

    match Doc::new(&path, src, &args) {
        Ok(doc) => DOCS_ARENA.with_borrow_mut(|arena| {
            let id = arena.push(doc);
            id as _
        }),

        Err(err) => match err {
            DocCreateError::NulError(_) => -1,
            DocCreateError::CXError(err) => -1 - err as isize,
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn doc_dispose(id: isize) {
    DOCS_ARENA.with_borrow_mut(|arena| {
        let _ = arena.take(id as usize);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn doc_update(id: isize, src: *const c_char) -> isize {
    let source_buf = CStrBuffer(unsafe { CStr::from_ptr(src) });

    DOCS_ARENA.with_borrow_mut(|arena| {
        if let Some(doc) = arena.get_mut(id as usize) {
            doc.replace_buf(source_buf);
            match doc.reparse() {
                Ok(()) => 0,
                Err(err) => -1 - err as isize,
            }
        } else {
            -1
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn doc_code_complete(
    id: isize,
    line: std::ffi::c_uint,
    col: std::ffi::c_uint,
) -> *const c_char {
    let result = DOCS_ARENA
        .with_borrow(|arena| {
            let Some(doc) = arena.get(id as usize) else {
                return None;
            };

            match doc.code_complete(line, col) {
                Some(compl) => {
                    let completions = compl.iter().collect::<Vec<_>>();
                    serde_json::to_string(&completions).ok()
                }
                None => None,
            }
        })
        .unwrap_or_default();

    // SAFETY: needs libc free() on caller side
    CString::new(result.replace('\0', "")).unwrap().into_raw()
}

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
fn main() {}

#[cfg(not(all(target_arch = "wasm32", feature = "wasm")))]
fn main() {
    panic!("compile this to webassembly please :)")
}
