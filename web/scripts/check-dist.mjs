// Checks the built page: WASM under 300 KB gzipped, a strict CSP that
// forbids network requests, and no remote URLs in the bundle.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { gzipSync } from "node:zlib";

const dist = resolve(import.meta.dirname, "../dist");
const wasm = readFileSync(resolve(import.meta.dirname, "../src/wasm/arbcalc_wasm_bg.wasm"));
const gz = gzipSync(wasm, { level: 9 }).length;
const fail = (m) => {
  console.error(`check-dist: ${m}`);
  process.exit(1);
};
if (gz > 300 * 1024) fail(`WASM is ${gz} bytes gzipped, over 300 KB`);

const html = readFileSync(join(dist, "index.html"), "utf8");
if (!html.includes("connect-src 'none'")) fail("CSP must set connect-src 'none'");
if (/<script[^>]+src="https?:/.test(html) || /<link[^>]+href="https?:/.test(html)) {
  fail("index.html loads a remote resource");
}
const files = [];
const walk = (d) =>
  readdirSync(d).forEach((f) => {
    const p = join(d, f);
    statSync(p).isDirectory() ? walk(p) : files.push(p);
  });
walk(dist);
let total = 0;
for (const f of files) {
  total += statSync(f).size;
  if (!/\.(js|css)$/.test(f)) continue;
  const text = readFileSync(f, "utf8");
  for (const m of text.matchAll(/(fetch\(|XMLHttpRequest|WebSocket|EventSource|sendBeacon)/g)) {
    fail(`${f} contains ${m[0]}`);
  }
}
console.log(`check-dist: WASM ${(gz / 1024).toFixed(1)} KB gzipped; ${files.length} files, ${(total / 1024).toFixed(0)} KB in dist; CSP ok`);
