//! Error conversion between Laurus errors and Python exceptions.

use laurus::LaurusError;
use pyo3::PyErr;
use pyo3::exceptions::{PyIOError, PyRuntimeError, PyValueError};

/// Convert a [`LaurusError`] into a Python exception.
pub fn laurus_err(err: LaurusError) -> PyErr {
    match err {
        LaurusError::Io(e) => PyIOError::new_err(e.to_string()),
        LaurusError::Schema(m) => PyValueError::new_err(format!("Schema error: {m}")),
        LaurusError::Query(m) => PyValueError::new_err(format!("Query error: {m}")),
        LaurusError::Field(m) => PyValueError::new_err(format!("Field error: {m}")),
        other => PyRuntimeError::new_err(other.to_string()),
    }
}
