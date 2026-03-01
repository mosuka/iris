//! Index management endpoints.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use super::GatewayState;
use super::convert;
use super::error::{BadRequest, GatewayError};
use crate::proto::laurus::v1;

/// `POST /v1/index` — Creates a new index.
pub async fn create(
    State(mut state): State<GatewayState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, Response> {
    let schema_json = body
        .get("schema")
        .ok_or_else(|| BadRequest("missing \"schema\" key".to_string()).into_response())?;

    let schema =
        convert::json_to_proto_schema(schema_json).map_err(|e| BadRequest(e).into_response())?;

    state
        .index_client
        .create_index(v1::CreateIndexRequest {
            schema: Some(schema),
        })
        .await
        .map_err(|s| GatewayError(s).into_response())?;

    Ok(Json(json!({})))
}

/// `GET /v1/index` — Returns index statistics.
pub async fn get_index(State(mut state): State<GatewayState>) -> Result<Json<Value>, Response> {
    let response = state
        .index_client
        .get_index(v1::GetIndexRequest {})
        .await
        .map_err(|s| GatewayError(s).into_response())?;

    let inner = response.into_inner();
    let vector_fields: serde_json::Map<String, Value> = inner
        .vector_fields
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                json!({
                    "vector_count": v.vector_count,
                    "dimension": v.dimension,
                }),
            )
        })
        .collect();

    Ok(Json(json!({
        "document_count": inner.document_count,
        "vector_fields": vector_fields,
    })))
}

/// `GET /v1/schema` — Returns the current schema.
pub async fn get_schema(State(mut state): State<GatewayState>) -> Result<Json<Value>, Response> {
    let response = state
        .index_client
        .get_schema(v1::GetSchemaRequest {})
        .await
        .map_err(|s| GatewayError(s).into_response())?;

    let inner = response.into_inner();
    let schema_json = inner
        .schema
        .as_ref()
        .map(convert::proto_schema_to_json)
        .unwrap_or(Value::Null);

    Ok(Json(json!({ "schema": schema_json })))
}
