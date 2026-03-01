//! Search endpoints (unary + SSE streaming).

use std::convert::Infallible;

use axum::Json;
use axum::extract::State;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use tokio_stream::StreamExt;

use super::GatewayState;
use super::convert;
use super::error::{BadRequest, GatewayError};

/// `POST /v1/search` — Executes a search and returns all results at once.
pub async fn search(
    State(mut state): State<GatewayState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, Response> {
    let request =
        convert::json_to_proto_search_request(&body).map_err(|e| BadRequest(e).into_response())?;

    let response = state
        .search_client
        .search(request)
        .await
        .map_err(|s| GatewayError(s).into_response())?;

    let inner = response.into_inner();
    let results: Vec<Value> = inner
        .results
        .iter()
        .map(convert::proto_search_result_to_json)
        .collect();

    Ok(Json(json!({ "results": results })))
}

/// `POST /v1/search/stream` — Executes a search and returns results incrementally via SSE.
pub async fn search_stream(
    State(mut state): State<GatewayState>,
    Json(body): Json<Value>,
) -> Response {
    let request = match convert::json_to_proto_search_request(&body) {
        Ok(r) => r,
        Err(e) => return BadRequest(e).into_response(),
    };

    let response = match state.search_client.search_stream(request).await {
        Ok(r) => r,
        Err(s) => return GatewayError(s).into_response(),
    };

    let grpc_stream = response.into_inner();

    let sse_stream = grpc_stream.map(|result| -> Result<Event, Infallible> {
        match result {
            Ok(search_result) => {
                let json = convert::proto_search_result_to_json(&search_result);
                let data = serde_json::to_string(&json).unwrap_or_default();
                Ok(Event::default().data(data))
            }
            Err(status) => {
                let error_json = json!({
                    "error": {
                        "code": status.code() as i32,
                        "message": status.message(),
                    }
                });
                let data = serde_json::to_string(&error_json).unwrap_or_default();
                Ok(Event::default().event("error").data(data))
            }
        }
    });

    Sse::new(sse_stream).into_response()
}
