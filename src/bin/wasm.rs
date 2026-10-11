// #![cfg(all(target_arch = "wasm32", feature = "wasm"))]
use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char};

use autoc::prelude::*;

#[derive(Default)]
struct Arena {
    items: Vec<Option<Doc>>,
    vacancies: Vec<usize>,
}

thread_local! { static DOCS_ARENA: RefCell<Arena> = RefCell::new(Arena::default()); }

/// Opens a document, returning either a negative error status code or
/// an ID of the document.
///
/// Error codes:
/// - `-1`: Null in string
/// - `-2`: Parsing failed
///
/// Use the returned ID as the "context," and always dispose the document
/// after its use.
#[unsafe(no_mangle)]
pub extern "C" fn doc_open(src: *const c_char, flags: *const c_char) -> i32 {
    let (src, flags) = unsafe {
        (
            CStr::from_ptr(src).to_str().unwrap(),
            CStr::from_ptr(flags).to_str().unwrap(),
        )
    };
    let args: Vec<&str> = flags.lines().collect();
    match Doc::new("main.cpp", src, &args) {
        Ok(doc) => DOCS_ARENA.with(|cell| {
            let mut cell = cell.borrow_mut();

            if let Some(vacancy) = cell.vacancies.pop() {
                cell.items
                    .get_mut(vacancy)
                    .expect("vacancy not found")
                    .replace(doc);
                vacancy as i32
            } else {
                cell.items.push(Some(doc));
                (cell.items.len() - 1) as i32
            }
        }),
        Err(err) => match err {
            DocError::NulInStr(_) => -1,
            DocError::ParseFailed => -2,
        },
    }
}

/// Attempt to update the document with the given source buffer.
///
/// If it succeeds, `0` is returned; otherwise, a negative error
/// is returned.
///
/// For the mapping, see [`autoc::prelude::DocUpdateError`].
#[unsafe(no_mangle)]
pub extern "C" fn doc_update(id: i32, src: *const c_char) -> i32 {
    let src = unsafe { CStr::from_ptr(src) }.to_str().unwrap();
    DOCS_ARENA.with(|cell| {
        let mut cell = cell.borrow_mut();
        cell.items
            .get_mut(id as usize)
            .map(|doc| {
                doc.as_mut()
                    .expect("unknown doc")
                    .update(src)
                    .map_or_else(|err| err as i32, |_| 0)
            })
            .unwrap_or(-1)
    })
}

/// Attempt to perform autocompletion on a document at the given
/// line and column.
///
/// JSON text is returned, and it must be freed after use.
#[unsafe(no_mangle)]
pub extern "C" fn doc_complete(id: i32, line: u32, col: u32) -> *mut c_char {
    let out = DOCS_ARENA.with(|cell| {
        let mut cell = cell.borrow_mut();
        let Some(doc) = cell.items.get_mut(id as usize) else {
            return String::from("[]");
        };
        let chunks = doc
            .as_mut()
            .expect("unknown doc")
            .complete(line, col)
            .collect::<Vec<_>>();
        serde_json::to_string(&chunks).unwrap_or_else(|_| "[]".to_string())
    });

    // use libc free on the JS side
    CString::new(out.replace('\0', "")).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn doc_dispose(id: i32) {
    DOCS_ARENA.with(|cell| {
        let mut cell = cell.borrow_mut();
        if let Some(doc) = cell.items.get_mut(id as usize) {
            doc.take();
            cell.vacancies.push(id as usize);
        }
    })
}

fn main() {}
