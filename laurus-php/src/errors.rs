//! Error conversion between Laurus errors and PHP exceptions.

use ext_php_rs::exception::PhpException;
use ext_php_rs::zend::ce;
use laurus::LaurusError;

/// Convert a [`LaurusError`] into a PHP exception.
///
/// # Mapping
///
/// | Laurus variant            | PHP exception    |
/// |---------------------------|------------------|
/// | `LaurusError::Io`         | `Exception`      |
/// | `LaurusError::Schema`     | `ValueError`     |
/// | `LaurusError::Query`      | `ValueError`     |
/// | `LaurusError::Field`      | `ValueError`     |
/// | other                     | `Exception`      |
pub fn laurus_err(err: LaurusError) -> PhpException {
    match err {
        LaurusError::Io(e) => PhpException::new(e.to_string(), 0, ce::exception()),
        LaurusError::Schema(m) => {
            PhpException::new(format!("Schema error: {m}"), 0, ce::value_error())
        }
        LaurusError::Query(m) => {
            PhpException::new(format!("Query error: {m}"), 0, ce::value_error())
        }
        LaurusError::Field(m) => {
            PhpException::new(format!("Field error: {m}"), 0, ce::value_error())
        }
        other => PhpException::new(other.to_string(), 0, ce::exception()),
    }
}
