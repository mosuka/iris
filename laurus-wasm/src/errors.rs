//! Error conversion between Laurus errors and wasm-bindgen JsValue errors.

use laurus::LaurusError;
use wasm_bindgen::JsValue;

/// Convert a [`LaurusError`] into a [`JsValue`] error.
///
/// # Arguments
///
/// * `err` - The Laurus error to convert.
///
/// # Returns
///
/// A `JsValue` containing the error message string.
pub fn laurus_err(err: LaurusError) -> JsValue {
    JsValue::from_str(&err.to_string())
}
