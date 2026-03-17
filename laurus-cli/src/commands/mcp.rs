//! MCP (Model Context Protocol) server launcher.
//!
//! Starts the `laurus-mcp` server on stdio, optionally connecting to a running
//! laurus-server gRPC instance at startup.  If no endpoint is given, the
//! server starts without a connection and the `connect` MCP tool can be used
//! to connect to a laurus-server later.

use anyhow::Result;

/// Start the MCP server on stdio.
///
/// Delegates to [`laurus_mcp::server::run`], which reads MCP requests from
/// stdin and writes responses to stdout.
///
/// # Arguments
///
/// * `endpoint` - Optional gRPC endpoint URL of a running laurus-server
///   (e.g. `http://localhost:50051`).
///
/// # Errors
///
/// Returns an error if the MCP server fails to start or encounters a fatal
/// runtime error.
pub async fn run(endpoint: Option<&str>) -> Result<()> {
    laurus_mcp::server::run(endpoint).await
}
