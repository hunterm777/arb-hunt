import { readFileSync } from "node:fs";
import { defineConfig, type Plugin } from "vite";

// Inlines a .wasm file as base64 text (`import b64 from "x.wasm?b64"`), so the
// page compiles its core from the bundle and never fetches it.
function wasmBase64(): Plugin {
  return {
    name: "wasm-base64",
    enforce: "pre",
    load(id) {
      if (!id.endsWith(".wasm?b64")) return null;
      const bytes = readFileSync(id.slice(0, -"?b64".length));
      return `export default ${JSON.stringify(bytes.toString("base64"))};`;
    },
  };
}

// The page is one static bundle: the WASM core is inlined into the script
// and the fonts ship from the same origin, so nothing is fetched after load.
export default defineConfig({
  base: "./",
  plugins: [wasmBase64()],
  build: {
    target: "es2022",
    assetsInlineLimit: 0,
    modulePreload: false,
    sourcemap: false,
  },
});
