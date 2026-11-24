//! Python bindings: JSON strings in and out of the core, nothing else.
//! Errors are raised as `ValueError`.

#![forbid(unsafe_code)]

use arbcalc_core::api;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Checks one arb (input JSON, as in the fixtures) and returns the report (JSON).
#[pyfunction]
fn check_json(input: &str) -> PyResult<String> {
    api::check_json(input).map_err(PyValueError::new_err)
}

/// The venue-specs version, fee categories per venue, and which fee rules are unverified (JSON).
#[pyfunction]
fn meta() -> String {
    api::meta()
}

/// The compiled core.
#[pymodule]
fn _arbcalc(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(check_json, m)?)?;
    m.add_function(wrap_pyfunction!(meta, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
