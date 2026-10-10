use std::{env, fs, path::PathBuf};

fn main() {
    let host_root = env::var("LLVM_ROOT").unwrap_or_else(|_| "/usr/lib/llvm-19".into());

    let inc = format!("{host_root}/include");

    cc::Build::new()
        .file("c/autoc.c")
        .include(&inc)
        .flag_if_supported("-std=gnu11")
        .compile("autoc");

    let mut bindgen_builder = bindgen::Builder::default()
        .header("c/autoc.h")
        .clang_arg(format!("-I{inc}"))
        .allowlist_function("autoc_.*")
        .allowlist_type("autoc_.*")
        .prepend_enum_name(false)
        .derive_debug(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    let is_wasm = env::var("TARGET").unwrap() == "wasm32-unknown-emscripten";

    if is_wasm {
        let emsdk = env::var("EMSDK").expect("source emsdk_env.sh");
        bindgen_builder = bindgen_builder
            .clang_arg("--target=wasm32-unknown-emscripten")
            .clang_arg(format!(
                "-isystem{emsdk}/upstream/emscripten/cache/sysroot/include"
            ));
    }

    bindgen_builder
        .generate()
        .expect("bindgen failed")
        .write_to_file(PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs"))
        .unwrap();

    if is_wasm {
        let lib = env::var("LIBCLANG_WASM_DIR").expect("set LIBCLANG_WASM_DIR to build-wasm/lib");
        println!("cargo:rustc-link-search=native={lib}");
        for e in fs::read_dir(&lib).unwrap().flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "a") {
                let name = p.file_stem().unwrap().to_str().unwrap();
                println!(
                    "cargo:rustc-link-lib=static={}",
                    name.trim_start_matches("lib")
                );
            }
        }
        println!("cargo:rustc-link-lib=c++");
    } else {
        let lib = format!("{host_root}/lib");
        println!("cargo:rustc-link-search=native={lib}");
        println!("cargo:rustc-link-lib=clang-19");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{lib}");
    }

    println!("cargo:rerun-if-changed=c/autoc.c");
    println!("cargo:rerun-if-env-changed=LLVM_ROOT");
    println!("cargo:rerun-if-env-changed=LIBCLANG_WASM_DIR");
}
