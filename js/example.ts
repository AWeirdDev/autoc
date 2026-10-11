import { Doc, initModule } from ".";

const source = `struct Point { int x; int y; void move69(int dx, int dy = 0); };
int main() { Point p; p.mo
}
`;

await initModule();

using doc = new Doc({
    source,
    flags: ["-x", "c++", "-std=c++20", "--target=wasm32-wasi"],
});

console.log(doc.complete(2, 26)[0]!.chunks);
