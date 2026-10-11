// needs /usr/lib/llvm-19/lib/clang/19/include

const createModule = require("./wasm");
const fs = require("fs");

const RESOURCE_SRC = "/usr/lib/llvm-19/lib/clang/19/include";
const VIRTUAL_FS = "/clang-resource/include";

(async () => {
    const module = await createModule();

    module.FS.mkdirTree(VIRTUAL_FS);
    for (const file of fs.readdirSync(RESOURCE_SRC)) {
        const path = `${RESOURCE_SRC}/${file}`;
        if (fs.statSync(path).isFile())
            module.FS.writeFile(VIRTUAL_FS + "/" + file, fs.readFileSync(path));
    }

    const withStr = (s, fn) => {
        const p = module.stringToNewUTF8(s);
        try {
            return fn(p);
        } finally {
            module._free(p);
        }
    };

    const src = `struct Point { int x; int y; void move69(int dx, int dy = 0); };
int main() { Point p; p.mo
}
`;
    const leakedSrc = module.stringToNewUTF8(src);

    const flags = ["-x", "c++", "-std=c++20", "--target=wasm32-wasi"].join(
        "\n",
    );
    const leakedFlags = module.stringToNewUTF8(flags);

    const result = module._doc_open(leakedSrc, leakedFlags);
    if (result !== 0) {
        throw new Error("failed to open doc, status=" + result);
    }

    const col = Buffer.byteLength("int main() { Point p; p.mo") + 1;
    const ptr = module._complete(2, col);
    console.log(module.UTF8ToString(ptr));

    module._free(ptr);
})();

// [/AI CONTENT]
