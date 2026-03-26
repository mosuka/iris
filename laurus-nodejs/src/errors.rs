//! Error conversion between Laurus errors and napi-rs errors.

use laurus::LaurusError;
use napi::Status;

/// Convert a [`LaurusError`] into a napi [`napi::Error`].
pub fn laurus_err(err: LaurusError) -> napi::Error {
    match err {
        LaurusError::Io(e) => napi::Error::new(Status::GenericFailure, format!("IO error: {e}")),
        LaurusError::Schema(m) => {
            napi::Error::new(Status::InvalidArg, format!("Schema error: {m}"))
        }
        LaurusError::Query(m) => napi::Error::new(Status::InvalidArg, format!("Query error: {m}")),
        LaurusError::Field(m) => napi::Error::new(Status::InvalidArg, format!("Field error: {m}")),
        other => napi::Error::new(Status::GenericFailure, other.to_string()),
    }
}
