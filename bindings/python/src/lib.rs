//! PyO3 boundary for the RSPDL Python package.

#![forbid(unsafe_code)]

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use rspdl_sdk::SdkError;

fn binding_error(error: SdkError) -> PyErr {
    PyRuntimeError::new_err(format!("{}: {error}", error.code()))
}

fn run_without_gil(
    py: Python<'_>,
    request: String,
    operation: fn(&str) -> Result<String, SdkError>,
) -> PyResult<String> {
    py.detach(move || operation(&request))
        .map_err(binding_error)
}

#[pyfunction]
fn compile_json(py: Python<'_>, request: String) -> PyResult<String> {
    run_without_gil(py, request, rspdl_sdk::compile_json)
}

#[pyfunction]
fn check_json(py: Python<'_>, request: String) -> PyResult<String> {
    run_without_gil(py, request, rspdl_sdk::check_json)
}

#[pyfunction]
fn find_model_json(py: Python<'_>, request: String) -> PyResult<String> {
    run_without_gil(py, request, rspdl_sdk::find_model_json)
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(compile_json, module)?)?;
    module.add_function(wrap_pyfunction!(check_json, module)?)?;
    module.add_function(wrap_pyfunction!(find_model_json, module)?)?;
    Ok(())
}
