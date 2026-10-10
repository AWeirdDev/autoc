use std::{env, path::PathBuf};

fn main() {
    let root = std::env::var("LLVM_ROOT").unwrap_or_else(|_| "/usr/lib/llvm-19".into());
    let inc = format!("{root}/include");
    let lib = format!("{root}/lib");

    cc::Build::new()
        .file("c/autoc.c")
        .include(&inc)
        .flag_if_supported("-std=gnu11")
        .compile("autoc");

    let bindings = bindgen::Builder::default()
        .header("c/autoc.h")
        .clang_arg(format!("-I{inc}"))
        .allowlist_function("autoc_.*")
        .allowlist_type("autoc_.*")
        .prepend_enum_name(false)
        .derive_debug(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("bindgen failed");

    bindings
        .write_to_file(PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs"))
        .unwrap();

    println!("cargo:rustc-link-search=native={lib}");
    println!("cargo:rustc-link-lib=clang-19");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{lib}");
    println!("cargo:rerun-if-changed=c/autoc.c");
    println!("cargo:rerun-if-env-changed=LLVM_ROOT");
}
