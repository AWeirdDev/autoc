add-toolchain:
    rustup target add wasm32-unknown-emscripten

emsdk:
    #!/bin/sh
    git clone https://github.com/emscripten-core/emsdk.git
    cd emsdk
    ./emsdk install latest
    ./emsdk activate latest
    @echo fish:  . ./emsdk_env.fish
    @echo sh:    source ./emsdk_env.sh

patch-cindexer:
    patch llvm-project/clang/tools/libclang/CIndexer.cpp ./patches/CIndexer.patch
    touch llvm-project/clang/tools/libclang/CIndexer.cpp

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

build-libclang:
    ninja -C build-wasm libclang.a

build:
    touch c/autoc.c
    LIBCLANG_WASM_DIR=./build-wasm/lib cargo build --bin wasm --features wasm --release --target wasm32-unknown-emscripten

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
    echo "dist moved to ./dist"
    echo "tarball moved to ./$TARBALL"

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

# . ~/items/emsdk/emsdk_env.fish
