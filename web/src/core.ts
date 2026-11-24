// The only door to the math: every number on the page comes from the Rust core,
// compiled to WebAssembly and inlined into this bundle (no fetch).
import { initSync, check as wCheck, meta as wMeta } from "./wasm/arbcalc_wasm.js";
import wasmBase64 from "./wasm/arbcalc_wasm_bg.wasm?b64";
import type { Input, Meta, Report } from "./types";

let ready = false;

/** Compiles the inlined module; call once before anything else. */
export function init(): void {
  if (ready) return;
  const bin = atob(wasmBase64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  initSync({ module: bytes });
  ready = true;
}

/** Runs one check. Errors are the core's one sentence on what to fix. */
export function check(input: Input): { ok: Report } | { err: string } {
  try {
    return { ok: JSON.parse(wCheck(JSON.stringify(input))) as Report };
  } catch (e) {
    return { err: typeof e === "string" ? e : String(e) };
  }
}

export function meta(): Meta {
  return JSON.parse(wMeta()) as Meta;
}
