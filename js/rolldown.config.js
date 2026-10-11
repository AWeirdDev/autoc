import { defineConfig } from "rolldown";
import { dts } from "rolldown-plugin-dts";

export default defineConfig({
    input: "index.ts",
    plugins: [dts()],
    output: {
        dir: "dist",
        format: "es",
        minify: true,
    },
    platform: "browser",
});
