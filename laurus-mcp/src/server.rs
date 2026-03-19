//! MCP server implementation for the laurus search engine.
//!
//! [`LaurusMcpServer`] is a [`rmcp::ServerHandler`] that proxies MCP tool calls
//! to a running laurus-server instance via gRPC.  Use [`run`] to start the
//! server on stdio.

use std::sync::Arc;

use anyhow::Context as _;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::schemars;
use rmcp::{ErrorData as McpError, ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::RwLock;
use tonic::transport::Channel;
use tracing::info;

use laurus_server::proto::laurus::v1::{
    AddDocumentRequest, AddFieldRequest, CommitRequest, CreateIndexRequest, DeleteDocumentsRequest,
    GetDocumentsRequest, GetIndexRequest, PutDocumentRequest, SearchRequest,
    document_service_client::DocumentServiceClient, index_service_client::IndexServiceClient,
    search_service_client::SearchServiceClient,
};

use crate::convert;

// ── Parameter structs ─────────────────────────────────────────────────────────

/// Parameters for the `connect` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ConnectParams {
    /// gRPC endpoint of the laurus-server to connect to.
    ///
    /// Must include the scheme and port, for example `http://localhost:50051`.
    endpoint: String,
}

/// Parameters for the `create_index` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CreateIndexParams {
    /// Schema definition as a JSON string.
    ///
    /// The schema must conform to the laurus schema format.  See the laurus
    /// documentation for the full field type reference
    /// (Text, Integer, Float, Boolean, DateTime, Geo, Hnsw, Flat, Ivf, …).
    ///
    /// FieldOption uses serde's externally-tagged representation where the
    /// variant name is the key.  Example:
    ///
    /// ```json
    /// {
    ///   "fields": {
    ///     "title": { "Text": { "indexed": true, "stored": true } },
    ///     "body":  { "Text": {} },
    ///     "score": { "Float": {} },
    ///     "vec":   { "Hnsw": { "dimension": 384 } }
    ///   }
    /// }
    /// ```
    schema_json: String,
}

/// Parameters for the `add_document` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AddDocumentParams {
    /// External document identifier (used for retrieval and deduplication).
    id: String,

    /// Document fields as a JSON object.
    ///
    /// Field names and value types must match the index schema.
    document: Value,

    /// Indexing mode.
    ///
    /// - `"put"` (default): upsert — delete any existing document with the
    ///   same `id` before indexing the new one.
    /// - `"add"`: append — allow multiple chunks to share the same `id`
    ///   (useful for splitting large documents into paragraphs / pages).
    mode: Option<String>,
}

/// Parameters for the `get_document` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetDocumentParams {
    /// External document identifier to look up.
    id: String,
}

/// Parameters for the `delete_document` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DeleteDocumentParams {
    /// External document identifier to delete.
    id: String,
}

/// Parameters for the `search` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// Search query string in the laurus query DSL.
    ///
    /// Supports term queries (`title:hello`), boolean operators (`AND`, `OR`,
    /// `NOT`), phrase queries (`"exact phrase"`), fuzzy queries (`roam~2`),
    /// range queries (`date:[2024-01-01 TO 2024-12-31]`), and more.
    query: String,

    /// Maximum number of results to return. Defaults to `10`.
    limit: Option<u32>,

    /// Number of results to skip for pagination. Defaults to `0`.
    offset: Option<u32>,
}

/// Parameters for the `add_field` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AddFieldParams {
    /// The name of the new field to add.
    name: String,

    /// Field configuration as a JSON string.
    ///
    /// Uses the same externally-tagged serde representation as the schema.
    /// The variant name is the key.  Example:
    ///
    /// ```json
    /// {"Text": {"indexed": true, "stored": true}}
    /// {"Hnsw": {"dimension": 384, "distance": "Cosine"}}
    /// {"Integer": {}}
    /// ```
    field_option_json: String,
}

// ── Server struct ─────────────────────────────────────────────────────────────

/// MCP server that proxies tool calls to a laurus-server gRPC instance.
///
/// The gRPC channel is stored in [`Arc<RwLock<Option<Channel>>>`].  When
/// `None`, no connection has been established yet; use the `connect` tool to
/// connect to a running laurus-server.
#[derive(Clone)]
pub struct LaurusMcpServer {
    channel: Arc<RwLock<Option<Channel>>>,
    tool_router: ToolRouter<LaurusMcpServer>,
}

// ── Tool implementations ───────────────────────────────────────────────────────

#[tool_router]
impl LaurusMcpServer {
    fn new(channel: Option<Channel>) -> Self {
        Self {
            channel: Arc::new(RwLock::new(channel)),
            tool_router: Self::tool_router(),
        }
    }

    /// Return a tool-level error result (not a protocol error).
    fn tool_error(msg: impl Into<String>) -> CallToolResult {
        CallToolResult::error(vec![Content::text(msg.into())])
    }

    // ── Connection tool ───────────────────────────────────────────────────────

    /// Connect to a running laurus-server gRPC endpoint.
    ///
    /// Call this tool before using any other tools when the MCP server was
    /// started without an `--endpoint` argument.  You can also call it to
    /// switch to a different laurus-server at any time.
    #[tool(
        description = "Connect to a laurus-server gRPC endpoint (e.g. http://localhost:50051). Call this before using other tools if the server was started without --endpoint."
    )]
    async fn connect(
        &self,
        Parameters(params): Parameters<ConnectParams>,
    ) -> Result<CallToolResult, McpError> {
        match Channel::from_shared(params.endpoint.clone())
            .map_err(|e| format!("{e}"))
            .map(|b| b.connect_lazy())
        {
            Ok(ch) => {
                *self.channel.write().await = Some(ch);
                info!("Connected to laurus-server at {}", params.endpoint);
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Connected to laurus-server at {}.",
                    params.endpoint
                ))]))
            }
            Err(e) => Ok(Self::tool_error(format!("Failed to connect: {e}"))),
        }
    }

    // ── Index tools ───────────────────────────────────────────────────────────

    /// Create a new search index with the provided schema.
    ///
    /// The schema describes the fields of the documents that will be indexed.
    #[tool(
        description = "Create a new search index with the provided schema. The schema_json must be a JSON string defining index fields (Text, Integer, Float, Boolean, DateTime, Hnsw, Flat, Ivf, etc.). Call this before add_document or search if the index does not exist yet."
    )]
    async fn create_index(
        &self,
        Parameters(params): Parameters<CreateIndexParams>,
    ) -> Result<CallToolResult, McpError> {
        let channel = match self.channel.read().await.clone() {
            Some(ch) => ch,
            None => {
                return Ok(Self::tool_error(
                    "Not connected. Call the connect tool first.",
                ));
            }
        };

        let laurus_schema: laurus::Schema = match serde_json::from_str(&params.schema_json) {
            Ok(s) => s,
            Err(e) => {
                return Ok(Self::tool_error(format!(
                    "Failed to parse schema JSON: {e}"
                )));
            }
        };

        let proto_schema = laurus_server::convert::schema::to_proto(&laurus_schema);
        let request = CreateIndexRequest {
            schema: Some(proto_schema),
        };

        match IndexServiceClient::new(channel).create_index(request).await {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                "Index created successfully.",
            )])),
            Err(e) => Ok(Self::tool_error(format!("Failed to create index: {e}"))),
        }
    }

    /// Get statistics for the open index.
    #[tool(
        description = "Get statistics for the current search index, including document count and vector field information."
    )]
    async fn get_index(&self) -> Result<CallToolResult, McpError> {
        let channel = match self.channel.read().await.clone() {
            Some(ch) => ch,
            None => {
                return Ok(Self::tool_error(
                    "Not connected. Call the connect tool first.",
                ));
            }
        };

        match IndexServiceClient::new(channel)
            .get_index(GetIndexRequest {})
            .await
        {
            Ok(resp) => {
                let r = resp.into_inner();
                let output = json!({
                    "document_count": r.document_count,
                    "vector_fields": r.vector_fields.keys().collect::<Vec<_>>(),
                });
                Ok(CallToolResult::success(vec![Content::text(
                    output.to_string(),
                )]))
            }
            Err(e) => Ok(Self::tool_error(format!("Failed to get index stats: {e}"))),
        }
    }

    /// Dynamically add a new field to the current index.
    #[tool(
        description = "Dynamically add a new field to an existing index. The field_option_json must be a JSON string describing the field type and options (e.g. '{\"Text\": {\"indexed\": true, \"stored\": true}}', '{\"Hnsw\": {\"dimension\": 384}}', '{\"Integer\": {}}'). Returns the updated schema."
    )]
    async fn add_field(
        &self,
        Parameters(params): Parameters<AddFieldParams>,
    ) -> Result<CallToolResult, McpError> {
        let channel = match self.channel.read().await.clone() {
            Some(ch) => ch,
            None => {
                return Ok(Self::tool_error(
                    "Not connected. Call the connect tool first.",
                ));
            }
        };

        let field_option: laurus::FieldOption =
            match serde_json::from_str(&params.field_option_json) {
                Ok(fo) => fo,
                Err(e) => {
                    return Ok(Self::tool_error(format!(
                        "Failed to parse field_option_json: {e}"
                    )));
                }
            };

        let proto_field_option =
            laurus_server::convert::schema::field_option_to_proto(&field_option);
        let request = AddFieldRequest {
            name: params.name.clone(),
            field_option: Some(proto_field_option),
        };

        match IndexServiceClient::new(channel).add_field(request).await {
            Ok(resp) => {
                let r = resp.into_inner();
                let output = if let Some(schema) = r.schema {
                    let field_names: Vec<&String> = schema.fields.keys().collect();
                    json!({
                        "message": format!("Field '{}' added successfully.", params.name),
                        "fields": field_names,
                    })
                } else {
                    json!({
                        "message": format!("Field '{}' added successfully.", params.name),
                    })
                };
                Ok(CallToolResult::success(vec![Content::text(
                    output.to_string(),
                )]))
            }
            Err(e) => Ok(Self::tool_error(format!("Failed to add field: {e}"))),
        }
    }

    // ── Document tools ────────────────────────────────────────────────────────

    /// Add or upsert a document in the index.
    #[tool(
        description = "Add a document to the index. Use mode='put' (default) to upsert (replace existing document with the same id), or mode='add' to append as a new chunk. Call commit after adding documents to persist changes."
    )]
    async fn add_document(
        &self,
        Parameters(params): Parameters<AddDocumentParams>,
    ) -> Result<CallToolResult, McpError> {
        let channel = match self.channel.read().await.clone() {
            Some(ch) => ch,
            None => {
                return Ok(Self::tool_error(
                    "Not connected. Call the connect tool first.",
                ));
            }
        };

        let doc = match convert::json_to_document(params.document) {
            Ok(d) => d,
            Err(e) => {
                return Ok(Self::tool_error(format!("Invalid document: {e}")));
            }
        };

        let mode = params.mode.as_deref().unwrap_or("put");
        let result = match mode {
            "add" => DocumentServiceClient::new(channel)
                .add_document(AddDocumentRequest {
                    id: params.id.clone(),
                    document: Some(doc),
                })
                .await
                .map(|_| ()),
            _ => DocumentServiceClient::new(channel)
                .put_document(PutDocumentRequest {
                    id: params.id.clone(),
                    document: Some(doc),
                })
                .await
                .map(|_| ()),
        };

        match result {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Document '{}' added. Call commit to persist changes.",
                params.id
            ))])),
            Err(e) => Ok(Self::tool_error(format!("Failed to add document: {e}"))),
        }
    }

    /// Get all stored documents for a given ID.
    #[tool(
        description = "Retrieve stored document(s) by external ID. Returns a JSON array of documents matching the ID."
    )]
    async fn get_document(
        &self,
        Parameters(params): Parameters<GetDocumentParams>,
    ) -> Result<CallToolResult, McpError> {
        let channel = match self.channel.read().await.clone() {
            Some(ch) => ch,
            None => {
                return Ok(Self::tool_error(
                    "Not connected. Call the connect tool first.",
                ));
            }
        };

        match DocumentServiceClient::new(channel)
            .get_documents(GetDocumentsRequest {
                id: params.id.clone(),
            })
            .await
        {
            Ok(resp) => {
                let json_docs: Vec<Value> = resp
                    .into_inner()
                    .documents
                    .iter()
                    .map(convert::document_to_json)
                    .collect();
                let output = json!({
                    "id": params.id,
                    "documents": json_docs,
                });
                Ok(CallToolResult::success(vec![Content::text(
                    output.to_string(),
                )]))
            }
            Err(e) => Ok(Self::tool_error(format!("Failed to get document: {e}"))),
        }
    }

    /// Delete all documents with the given external ID.
    #[tool(
        description = "Delete document(s) by external ID from the index. Removes all chunks sharing the same ID. Call commit to persist changes."
    )]
    async fn delete_document(
        &self,
        Parameters(params): Parameters<DeleteDocumentParams>,
    ) -> Result<CallToolResult, McpError> {
        let channel = match self.channel.read().await.clone() {
            Some(ch) => ch,
            None => {
                return Ok(Self::tool_error(
                    "Not connected. Call the connect tool first.",
                ));
            }
        };

        match DocumentServiceClient::new(channel)
            .delete_documents(DeleteDocumentsRequest {
                id: params.id.clone(),
            })
            .await
        {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Document '{}' deleted. Call commit to persist changes.",
                params.id
            ))])),
            Err(e) => Ok(Self::tool_error(format!("Failed to delete document: {e}"))),
        }
    }

    /// Commit pending changes to disk.
    #[tool(
        description = "Commit pending changes to disk. Must be called after add_document or delete_document to make changes searchable and durable."
    )]
    async fn commit(&self) -> Result<CallToolResult, McpError> {
        let channel = match self.channel.read().await.clone() {
            Some(ch) => ch,
            None => {
                return Ok(Self::tool_error(
                    "Not connected. Call the connect tool first.",
                ));
            }
        };

        match DocumentServiceClient::new(channel)
            .commit(CommitRequest {})
            .await
        {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                "Changes committed successfully.",
            )])),
            Err(e) => Ok(Self::tool_error(format!("Failed to commit: {e}"))),
        }
    }

    // ── Search tools ──────────────────────────────────────────────────────────

    /// Search documents using the laurus query DSL.
    #[tool(
        description = "Search documents using the laurus query DSL. Supports term queries (field:value), boolean operators (AND, OR, NOT), phrase queries (\"exact phrase\"), fuzzy queries (term~2), and range queries (field:[from TO to]). Returns a JSON array of results with id, score, and document fields."
    )]
    async fn search(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let channel = match self.channel.read().await.clone() {
            Some(ch) => ch,
            None => {
                return Ok(Self::tool_error(
                    "Not connected. Call the connect tool first.",
                ));
            }
        };

        let request = SearchRequest {
            query: params.query,
            limit: params.limit.unwrap_or(10),
            offset: params.offset.unwrap_or(0),
            ..Default::default()
        };

        match SearchServiceClient::new(channel).search(request).await {
            Ok(resp) => {
                let r = resp.into_inner();
                let json_results: Vec<Value> = r
                    .results
                    .iter()
                    .map(|result| {
                        json!({
                            "id": result.id,
                            "score": result.score,
                            "document": result.document.as_ref().map(convert::document_to_json),
                        })
                    })
                    .collect();

                let output = json!({
                    "total": r.total_hits,
                    "results": json_results,
                });
                Ok(CallToolResult::success(vec![Content::text(
                    output.to_string(),
                )]))
            }
            Err(e) => Ok(Self::tool_error(format!("Search failed: {e}"))),
        }
    }
}

// ── ServerHandler impl ────────────────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for LaurusMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions(
                "Laurus search engine MCP server (gRPC client). \
             Tools: connect, create_index, get_index, add_field, add_document, get_document, \
             delete_document, commit, search. \
             Start by calling connect(endpoint) to connect to a running laurus-server, \
             then use the other tools to manage and search the index."
                    .to_string(),
            )
    }
}

// ── Public entry point ─────────────────────────────────────────────────────────

/// Start the MCP server on stdio.
///
/// If `endpoint` is provided, connects to the laurus-server immediately.
/// Otherwise the server starts without a connection; use the `connect` tool
/// to connect to a running laurus-server before using other tools.
///
/// This function runs until stdin is closed or an unrecoverable error occurs.
///
/// # Arguments
///
/// * `endpoint` - Optional gRPC endpoint URL (e.g. `http://localhost:50051`).
///
/// # Errors
///
/// Returns an error if the server transport fails to start or encounters a
/// fatal runtime error.
pub async fn run(endpoint: Option<&str>) -> anyhow::Result<()> {
    let channel = if let Some(ep) = endpoint {
        info!("Connecting to laurus-server at {ep}");
        let ch = Channel::from_shared(ep.to_string())
            .context("Invalid endpoint URI")?
            .connect_lazy();
        Some(ch)
    } else {
        info!("No endpoint specified. Use the connect tool to connect to a laurus-server.");
        None
    };

    let server = LaurusMcpServer::new(channel);
    let transport = (tokio::io::stdin(), tokio::io::stdout());
    let service = server
        .serve(transport)
        .await
        .context("Failed to start MCP server")?;

    service.waiting().await.context("MCP server error")?;

    Ok(())
}
