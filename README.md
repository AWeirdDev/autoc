# autoc
Autocomplete for C projects.

## C API
See `./c/autoc.c` and `./c/autoc.h`.

## Building
You need to first download LLVM source as `llvm-project`, with CMake, Ninja, and Emscripten, and [Just](https://just.systems) installed.
Then, run:

```sh
just build-llvm && just build-wasm
```
