# autoc
Autocomplete for C projects.

## C API
See `./c/autoc.c` and `./c/autoc.h`.

## Building for WASM
You need to first download LLVM source as `llvm-project`, with CMake, Ninja, and Emscripten, and [Just](https://just.systems) installed.
Then, run:

```sh
just add-toolchain # rust toolchain
just emsdk # prepare emsdk

# read the output, choose your command
# according to your terminal

just patch-indexer # patch for wasm
just build-llvm && just build-libclang

# and then our library
just build && just dist

# test the dist/
just test-dist
```

It's kind of complicated, but I guess it's worth it. I've already compiled the WASM version to `dist/` and uploaded it to the repository, so you can use it directly.
