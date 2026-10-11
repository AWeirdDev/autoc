# autoc
Autocomplete for C projects.

autoc embeds libclang, and can be compiled to WebAssembly or be used as a C/Rust library.
Originally, this was going to be C-only, but I sucked at C so much that I decided to just write the WebAssembly exports in Rust.

The idea is:

```c
#include "autoc.h"
#include <string.h>

const char *source = "in";
autoc_Doc doc = autoc_Doc_new(
  "main.c",       // file
  strlen(source), // length
  source,         // buffer
  0,              // argc
  NULL,           // argv
);

// use autocomplete...

autoc_Doc_dispose(doc);
```

Seems simple enough. I hope.

## C API
See `./c/autoc.c` and `./c/autoc.h`.

## Building for WASM
You need to first download LLVM source as `llvm-project`, with CMake, Ninja, Emscripten, and [Just](https://just.systems) installed.
Then, run:

```sh
just add-toolchain # rust toolchain
just emsdk # prepare emsdk

# read the output, choose your command
# according to your terminal

just patch-cindexer # patch for wasm
just build-llvm && just build-libclang

# and then our library
just build && just dist

# test the dist/
just test-dist
```

It's kind of complicated, but I guess it's worth it. I've already compiled the WASM version to `dist/` and uploaded it to the repository, so you can use it directly.
