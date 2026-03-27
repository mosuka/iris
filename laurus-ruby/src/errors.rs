//! Error conversion between Laurus errors and Ruby exceptions.

use laurus::LaurusError;
use magnus::{Error, Ruby};

/// Convert a [`LaurusError`] into a Ruby exception.
///
/// # Mapping
///
/// | Laurus variant            | Ruby exception    |
/// |---------------------------|-------------------|
/// | `LaurusError::Io`         | `IOError`         |
/// | `LaurusError::Schema`     | `ArgumentError`   |
/// | `LaurusError::Query`      | `ArgumentError`   |
/// | `LaurusError::Field`      | `ArgumentError`   |
/// | other                     | `RuntimeError`    |
pub fn laurus_err(err: LaurusError) -> Error {
    let ruby = Ruby::get().expect("called from Ruby thread");
    match err {
        LaurusError::Io(e) => Error::new(ruby.exception_io_error(), e.to_string()),
        LaurusError::Schema(m) => {
            Error::new(ruby.exception_arg_error(), format!("Schema error: {m}"))
        }
        LaurusError::Query(m) => {
            Error::new(ruby.exception_arg_error(), format!("Query error: {m}"))
        }
        LaurusError::Field(m) => {
            Error::new(ruby.exception_arg_error(), format!("Field error: {m}"))
        }
        other => Error::new(ruby.exception_runtime_error(), other.to_string()),
    }
}
