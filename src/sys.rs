// src/sys.rs
#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    dead_code
)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    pub fn autoc_Doc_new(
        file: *const ::std::os::raw::c_char,
        len: ::std::os::raw::c_ulong,
        buf: *const ::std::os::raw::c_char,
        argc: ::std::os::raw::c_int,
        args: *const *const ::std::os::raw::c_char,
    ) -> autoc_Doc;
    pub fn autoc_Doc_reparse(doc: *mut autoc_Doc) -> autoc_Doc_ReparseResult;
    pub fn autoc_Doc_dispose(doc: autoc_Doc);
    pub fn autoc_Doc_code_complete(
        doc: *mut autoc_Doc,
        line: ::std::os::raw::c_uint,
        column: ::std::os::raw::c_uint,
    ) -> autoc_Completions;
    pub fn autoc_Completions_next(
        completions: *mut autoc_Completions,
        out: *mut autoc_Completion,
    ) -> ::std::os::raw::c_int;
    pub fn autoc_Completion_free(completion: autoc_Completion);
    pub fn autoc_Completions_dispose(completions: autoc_Completions);
}
