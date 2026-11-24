// Runs the shared fixture suite against the WASM build (Node target) and requires the
// exact text that native Rust gives, byte for byte, errors included.
// Usage: node crates/wasm/tests/parity.mjs <path to wasm-bindgen nodejs pkg>
import { readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { createRequire } from "node:module";

const root = resolve(import.meta.dirname, "../../..");
const pkg = resolve(process.argv[2] ?? join(root, "crates/wasm/pkg-node"));
const ac = createRequire(import.meta.url)(join(pkg, "arbcalc_wasm.js"));

function run(text) {
  try {
    return ac.check(text);
  } catch (e) {
    // Same shape as the native suite: {"error": message}.
    return JSON.stringify({ error: String(e) });
  }
}

const dir = join(root, "fixtures/cases");
const names = readdirSync(dir).filter((f) => f.endsWith(".json")).sort();
let failed = 0;
for (const name of names) {
  const got = run(readFileSync(join(dir, name), "utf8"));
  const want = readFileSync(join(root, "fixtures/expected", name), "utf8").trimEnd();
  if (got !== want) {
    failed++;
    let i = 0;
    while (got[i] === want[i]) i++;
    console.log(`FAIL ${name} at char ${i}: ${got.slice(i, i + 60)} | ${want.slice(i, i + 60)}`);
  } else {
    console.log(`ok   ${name}`);
  }
}
JSON.parse(ac.meta());
console.log(`${names.length - failed}/${names.length} fixtures match native Rust byte for byte`);
process.exit(failed ? 1 : 0);
