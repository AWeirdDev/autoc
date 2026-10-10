#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char};

use autoc::prelude::*;

thread_local! { static DOC: RefCell<Option<Doc>> = RefCell::new(None); }

#[unsafe(no_mangle)]
pub extern "C" fn doc_open(src: *const c_char, flags: *const c_char) -> i32 {
    let (src, flags) = unsafe {
        (
            CStr::from_ptr(src).to_str().unwrap(),
            CStr::from_ptr(flags).to_str().unwrap(),
        )
    };
    let args: Vec<&str> = flags.lines().collect();
    match Doc::new("/main.cpp", src, &args) {
        Ok(d) => {
            DOC.with(|c| *c.borrow_mut() = Some(d));
            0
        }
        Err(_) => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn doc_update(src: *const c_char) -> i32 {
    let src = unsafe { CStr::from_ptr(src) }.to_str().unwrap();
    DOC.with(|c| match c.borrow_mut().as_mut() {
        Some(d) => d.update(src).map(|_| 0).unwrap_or(-1),
        None => -1,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn complete(line: u32, col: u32) -> *mut c_char {
    let out = DOC.with(|c| {
        let mut b = c.borrow_mut();
        let Some(d) = b.as_mut() else {
            return String::new();
        };
        let mut s = String::new();
        for c in d.complete(line, col) {
            if let Some(CompletionChunk::TypedText(t)) = c
                .chunks
                .iter()
                .find(|x| matches!(x, CompletionChunk::TypedText(_)))
            {
                s += &format!("{t}\t{}\n", c.priority);
            }
        }
        s
    });
    // use libc free on the JS side
    CString::new(out.replace('\0', "")).unwrap().into_raw()
}

fn main() {}
