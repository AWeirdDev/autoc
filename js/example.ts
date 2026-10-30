import { createAutoC, CXCursorKind } from ".";

const SOURCE = `#include <str`;

const autoc = await createAutoC();
using doc = autoc.createDoc({
    path: "main.c",
    source: SOURCE,
    flags: ["-x", "c++", "-std=c++20", "--target=wasm32-wasi"],
});

console.log(
    doc
        .codeComplete(1, 13)
        .filter(
            (item) =>
                ![
                    CXCursorKind.NotImplemented,
                    CXCursorKind.MacroDefinition,
                ].includes(item.cursorKind),
        )
        .sort((a, b) => a.priority - b.priority)
        .slice(10)
        .map((item) => CXCursorKind[item.cursorKind]),
);
