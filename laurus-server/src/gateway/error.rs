//! Conversion from gRPC `Status` to HTTP responses.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use tonic::Code;

/// Wrapper type that converts a gRPC error into an HTTP response.
pub struct GatewayError(pub tonic::Status);

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let status = &self.0;
        let http_code = grpc_code_to_http(status.code());
        let body = json!({
            "error": {
                "code": http_code.as_u16(),
                "message": status.message(),
            }
        });
        (http_code, Json(body)).into_response()
    }
}

/// Wrapper type that converts conversion errors (e.g. JSON parse errors) into HTTP 400 responses.
pub struct BadRequest(pub String);

impl IntoResponse for BadRequest {
    fn into_response(self) -> Response {
        let body = json!({
            "error": {
                "code": 400,
                "message": self.0,
            }
        });
        (StatusCode::BAD_REQUEST, Json(body)).into_response()
    }
}

/// Converts a gRPC status code to an HTTP status code.
fn grpc_code_to_http(code: Code) -> StatusCode {
    match code {
        Code::Ok => StatusCode::OK,
        Code::InvalidArgument => StatusCode::BAD_REQUEST,
        Code::NotFound => StatusCode::NOT_FOUND,
        Code::AlreadyExists => StatusCode::CONFLICT,
        Code::PermissionDenied => StatusCode::FORBIDDEN,
        Code::FailedPrecondition => StatusCode::PRECONDITION_FAILED,
        Code::Unimplemented => StatusCode::NOT_IMPLEMENTED,
        Code::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        Code::Unauthenticated => StatusCode::UNAUTHORIZED,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
