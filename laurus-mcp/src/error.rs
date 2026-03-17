//! Error types for the laurus MCP server.

use thiserror::Error;

/// Errors that can occur in the laurus MCP server.
#[derive(Debug, Error)]
pub enum Error {
    /// The MCP server is not connected to a laurus-server instance.
    #[error("Not connected. Call the connect tool to connect to a laurus-server endpoint.")]
    NotConnected,

    /// A gRPC transport or connection error.
    #[error("gRPC transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    /// A gRPC call returned a non-OK status.
    #[error("gRPC error: {0}")]
    Status(#[from] tonic::Status),

    /// A JSON serialization or deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
