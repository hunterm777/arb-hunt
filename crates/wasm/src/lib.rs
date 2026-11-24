//! WebAssembly bindings: JSON strings in and out of the core, nothing else.
//! Errors come back as thrown strings.

#![forbid(unsafe_code)]

use arbcalc_core::api;
use wasm_bindgen::prelude::*;

/// Checks one arb (input JSON, as in the fixtures) and returns the report (JSON).
#[wasm_bindgen]
pub fn check(input: &str) -> Result<String, JsValue> {
    api::check_json(input).map_err(|e| JsValue::from_str(&e))
}

/// The venue-specs version, fee categories per venue, and which fee rules are unverified (JSON).
#[wasm_bindgen]
pub fn meta() -> String {
    api::meta()
}
