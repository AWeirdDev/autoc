// test.js  (run: node test.js)
const createModule = require("./wasm");

(async () => {
    const M = await createModule();

    // Check that the Rust exports survived linking.
    console.log(
        Object.keys(M).filter((k) => k.startsWith("_doc") || k === "_complete"),
    );

    const withStr = (s, fn) => {
        const p = M.stringToNewUTF8(s);
        try {
            return fn(p);
        } finally {
            M._free(p);
        }
    };

    const src = `struct Point { int x; int y; void move(int dx, int dy = 0); };
int main() { Point p; p.
}
`;
    const flags = ["-x", "c++", "-std=c++20", "--target=wasm32-wasi"].join(
        "\n",
    );

    const rc = withStr(src, (s) => withStr(flags, (f) => M._doc_open(s, f)));
    console.log("doc_open:", rc);

    // Line 2, column right after "p." (1-based, UTF-8 bytes).
    const col = Buffer.byteLength("int main() { Point p; p.") + 1;
    const ptr = M._complete(2, col);
    console.log(M.UTF8ToString(ptr));
    M._free(ptr);
})();
