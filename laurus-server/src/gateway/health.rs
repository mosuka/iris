//! Health check endpoint.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use super::GatewayState;
use super::error::GatewayError;
use crate::proto::laurus::v1;

/// `GET /v1/health` — Returns the server's serving status.
pub async fn check(State(mut state): State<GatewayState>) -> Result<Json<Value>, Response> {
    let response = state
        .health_client
        .check(v1::HealthCheckRequest {})
        .await
        .map_err(|s| GatewayError(s).into_response())?;

    let status = response.into_inner().status;
    let status_str = match v1::ServingStatus::try_from(status) {
        Ok(v1::ServingStatus::Serving) => "SERVING",
        Ok(v1::ServingStatus::NotServing) => "NOT_SERVING",
        _ => "UNKNOWN",
    };

    Ok(Json(json!({ "status": status_str })))
}
