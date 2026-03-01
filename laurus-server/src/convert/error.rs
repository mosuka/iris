use laurus::LaurusError;
use tonic::Status;

/// Convert a LaurusError into a tonic Status.
pub fn to_status(err: LaurusError) -> Status {
    match &err {
        LaurusError::Schema(_) | LaurusError::Query(_) | LaurusError::Field(_) => {
            Status::invalid_argument(err.to_string())
        }
        LaurusError::SerializationError(_) | LaurusError::Json(_) => {
            Status::invalid_argument(err.to_string())
        }
        LaurusError::NotImplemented(_) => Status::unimplemented(err.to_string()),
        _ => Status::internal(err.to_string()),
    }
}

/// Convert an anyhow::Error into a tonic Status.
pub fn anyhow_to_status(err: anyhow::Error) -> Status {
    Status::internal(err.to_string())
}
