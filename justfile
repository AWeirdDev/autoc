# Initializes the environment
init:
    @echo "=====> adding rust toolchain"
    just add-toolchain

    @echo "=====> getting emsdk, llvm"
    just get-emsdk & just get-llvm
    wait

    @echo "=====> patching CIndexer.h for wasm"
    just patch-cindexer

    @echo "=====> building llvm (native, wasm)"
    just build-llvm

    @echo "=====> building libclang"
    just build-libclang

    @echo "=====> building autoc"
    just build-autoc

    @echo "=====> done."
    @echo "       after init, you can use \`just dev\` whenever."

# Adds a Rust toolchain
add-toolchain:
    rustup target add wasm32-unknown-emscripten

# Gets emsdk to ./emsdk
get-emsdk:
    git init emsdk
    git -C emsdk fetch --depth 1 https://github.com/emscripten-core/emsdk.git
    git -C emsdk checkout FETCH_HEAD
    ./emsdk/emsdk install latest
    ./emsdk/emsdk activate latest
    @echo fish:  . ./emsdk_env.fish
    @echo sh:    source ./emsdk_env.sh

# Gets LLVM to ./llvm-project
get-llvm:
    # release/19.x
    git init llvm-project
    git -C llvm-project fetch --depth 1 https://github.com/llvm/llvm-project.git \
        "$(git ls-remote https://github.com/llvm/llvm-project.git "refs/heads/release/19.x" "refs/tags/release/19.x" | head -n1 | cut -f1)"
    git -C llvm-project checkout FETCH_HEAD

# Patch (...)/libclang/CIndexer.cpp for wasm
patch-cindexer:
    patch llvm-project/clang/tools/libclang/CIndexer.cpp ./patches/CIndexer.patch
    touch llvm-project/clang/tools/libclang/CIndexer.cpp

# Builds LLVM for wasm and native
build-llvm:
    cmake -S llvm-project/llvm -B build-native -G Ninja -DLLVM_ENABLE_PROJECTS=clang -DCMAKE_BUILD_TYPE=Release
    ninja -C build-native llvm-tblgen clang-tblgen

    emcmake cmake -S llvm-project/llvm -B build-wasm -G Ninja \
      -DCMAKE_BUILD_TYPE=MinSizeRel \
      -DLLVM_ENABLE_PROJECTS=clang \
      -DLLVM_TARGETS_TO_BUILD=WebAssembly \
      -DLLVM_NATIVE_TOOL_DIR=$PWD/build-native/bin \
      -DLLVM_ENABLE_THREADS=OFF -DLLVM_ENABLE_PIC=OFF \
      -DLLVM_ENABLE_TERMINFO=OFF -DLLVM_ENABLE_ZLIB=OFF -DLLVM_ENABLE_ZSTD=OFF \
      -DLLVM_INCLUDE_TESTS=OFF -DLLVM_INCLUDE_BENCHMARKS=OFF \
      -DLLVM_BUILD_TOOLS=OFF \
      -DCLANG_BUILD_TOOLS=ON -DCLANG_TOOL_LIBCLANG_BUILD=ON -DLIBCLANG_BUILD_STATIC=ON \
      -DCLANG_ENABLE_ARCMT=OFF -DCLANG_ENABLE_STATIC_ANALYZER=OFF \
      -DCMAKE_C_FLAGS="-fwasm-exceptions -sSUPPORT_LONGJMP=wasm" \
      -DCMAKE_CXX_FLAGS="-fwasm-exceptions -sSUPPORT_LONGJMP=wasm"
    ninja -C build-wasm -t targets all

# Builds libclang
build-libclang:
    ninja -C build-wasm libclang.a

# Builds autoc
build-autoc:
    cd emsdk && . ./emsdk_env.sh \
        && cd ../ \
        && LIBCLANG_WASM_DIR=$(realpath ./build-wasm/lib) \
           cargo build --bin wasm --features wasm --release --target wasm32-unknown-emscripten

# Generates dist for JavaScript
js-dist:
    #!/bin/sh
    set -xe
    cp ./target/wasm32-unknown-emscripten/release/wasm.* ./js
    rm ./js/*.d  # we dont need this
    cd js
    bun run build:clean && bun run build:browser
    TARBALL=$(bun pm pack --quiet)
    cd ../
    rm -rf ./dist/*
    mv ./js/dist .
    mv ./js/$TARBALL .
    mv ./js/*.wasm ./dist

# Tests autoc.h (legacy)
test-c:
    cc -Wall -Wextra \
        -I/usr/lib/llvm-19/include \
        -Ic \
        c/example.c c/autoc.c \
        -o example \
        -L/usr/lib/llvm-19/lib \
        -lclang \
        -Wl,-rpath,/usr/lib/llvm-19/lib
    ./example
    rm ./example

# Dev mode (after init)
dev:
    touch c/autoc.c
    just build-autoc

# Cleans the whole repo for any builds, emsdk, etc.
clean:
    rm -rf ./build-native ./build-wasm ./dist ./llvm-project ./emsdk
    rm -f "$(ls *.tgz 2>/dev/null)"
    cargo clean

# . ~/items/emsdk/emsdk_env.fish
