interface EmscriptenFS {
    writeFile(path: string, data: string | ArrayBufferView): void;
    mkdirTree(path: string): void;
    unlink(path: string): void;
}

interface EmscriptenModule {
    // emscripten rt
    FS: EmscriptenFS;
    UTF8ToString(ptr: number, maxBytesToRead?: number): string;
    stringToNewUTF8(str: string): number;

    // malloc
    _free(id: number): void;

    _doc_open(srcPtr: number, flagsPtr: number): (number & {}) | DocOpenError;
    _doc_complete(id: number, line: number, col: number): number;
    _doc_dispose(id: number): void;
}

enum DocOpenError {
    NullInString = -1,
    ParseFailed = -2,
}

export type CompletionChunk =
    | { type: "typedtext"; data: string }
    | { type: "text"; data: string }
    | { type: "resulttype"; data: string }
    | { type: "placeholder"; data: string }
    | { type: "currentparam"; data: string }
    | { type: "informative"; data: string }
    | { type: "punct"; data: string }
    | { type: "optional"; data: CompletionChunk[] };

export interface Completion {
    chunks: CompletionChunk[];
    priority: number;
    cursor_kind: number;
    brief: string | null;
}

const VIRTUAL_FS = "/clang-resource/include";
let MODULE: EmscriptenModule | null = null;

namespace Doc {
    export interface InitOptions {
        /**
         * The initial source text.
         */
        source: string;

        /**
         * The flags to use.
         */
        flags?: string[];
    }
}

/**
 * A source document.
 *
 * @example
 * ```ts
 * import { Doc, initModule } from ".";
 *
 * const source = `struct Point { int x; int y; void move69(int dx, int dy = 0); };
 * int main() { Point p; p.mo
 * }
 * `;
 *
 * await initModule();
 *
 * using doc = new Doc({
 *     source,
 *     flags: ["-x", "c++", "-std=c++20", "--target=wasm32-wasi"],
 * });
 *
 * console.log(doc.complete(2, 26));
 * ```
 *
 * Beware that resource allocation needs freeing. If the `using` keyword is unavailable,
 * fall back to `dispose()`:
 *
 * ```ts
 * let doc = new Doc({ ... });
 * doc.dispose();
 * ```
 */
export class Doc {
    #id: number;
    #srcPtr: number;
    #flagsPtr: number;

    constructor({ source, flags }: Doc.InitOptions) {
        const sourcePtr = unsafeCreateStringResource(source);
        const flagsPtr = unsafeCreateStringResource(
            flags ? flags.join("\n") : "",
        );

        const res = MODULE!._doc_open(sourcePtr, flagsPtr);
        switch (res) {
            case DocOpenError.NullInString: {
                this.dispose();
                throw new Error("Found null byte (\\0) in provided source");
            }

            case DocOpenError.ParseFailed: {
                this.dispose();
                throw new Error("clang failed to parse the source");
            }

            default: {
                this.#id = res;
                this.#srcPtr = sourcePtr;
                this.#flagsPtr = flagsPtr;
            }
        }
    }

    /**
     * Updates the source text.
     */
    update(source: string) {
        MODULE!._free(this.#srcPtr); // flush
        this.#srcPtr = unsafeCreateStringResource(source);
    }

    /**
     * Trigger autocompletion.
     */
    complete(line: number, col: number): Completion[] {
        const ptr = MODULE!._doc_complete(this.#id, line, col);
        const results: Completion[] = JSON.parse(MODULE!.UTF8ToString(ptr));
        MODULE!._free(ptr);
        return results;
    }

    /**
     * Dispose the document resource.
     * This is required to free memory.
     */
    dispose() {
        if (this.#id > 0) MODULE!._doc_dispose(this.#id);
        MODULE!._free(this.#flagsPtr);
        MODULE!._free(this.#srcPtr);
        this.#id = -1;
    }

    [Symbol.dispose]() {
        this.dispose();
    }
}

namespace initModule {
    interface InputFile {
        /**
         * The file path.
         */
        path: string;

        /**
         * The file data.
         */
        data: string | ArrayBufferView;
    }
    type InputFiles = InputFile[];

    export interface InitOptions {
        files?: InputFiles;
    }
}

/**
 * Initializes the autoc WASM module.
 *
 * The provided files will be put into the included headers
 * for clang to look for.
 */
export async function initModule(options?: initModule.InitOptions) {
    const module = await // @ts-ignore
    (await import("./wasm")).default();
    MODULE = module as EmscriptenModule;

    MODULE.FS.mkdirTree(VIRTUAL_FS);

    const { files } = options || {};
    if (files) {
        for (const { path, data } of files) {
            const virtualPath =
                VIRTUAL_FS +
                "/" +
                (path.startsWith("/") ? path.slice(1) : path);
            MODULE.FS.writeFile(virtualPath, data);
        }
    }
}

function unsafeCreateStringResource(str: string): number {
    return MODULE!.stringToNewUTF8(str);
}

// optional helpers

export function getTypedText(c: Completion): string {
    const t = c.chunks.find(
        (x): x is Extract<CompletionChunk, { type: "typedtext" }> =>
            x.type === "typedtext",
    );
    return t?.data ?? "";
}

export function getSignature(
    chunks: CompletionChunk[],
    showOptional = false,
): string {
    let out = "";
    for (const c of chunks) {
        switch (c.type) {
            case "resulttype":
                break;
            case "optional":
                if (showOptional) out += "[" + getSignature(c.data, true) + "]";
                break;
            default:
                out += c.data;
        }
    }
    return out;
}

export function getResultType(c: Completion): string | undefined {
    const r = c.chunks.find(
        (x): x is Extract<CompletionChunk, { type: "resulttype" }> =>
            x.type === "resulttype",
    );
    return r?.data;
}
