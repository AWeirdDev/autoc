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
# First, initialize your environment (takes a while)
# You can see the output messages to see what the
# script is doing
just init

# After that, use this command whenever you're done
# with some development
just dev

# If you need JavaScript bindings, use this
just js-dist
```
