//! Index management gRPC service.
//!
//! Handles index creation, metadata retrieval, and schema inspection through
//! the `IndexService` gRPC trait.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

use laurus::Engine;

use crate::context;
use crate::convert::{error, schema as schema_convert};
use crate::proto::laurus::v1::{
    CreateIndexRequest, CreateIndexResponse, GetIndexRequest, GetIndexResponse, GetSchemaRequest,
    GetSchemaResponse, VectorFieldStats, index_service_server::IndexService as IndexServiceTrait,
};

/// gRPC IndexService implementation.
#[derive(Clone)]
pub struct IndexService {
    /// Shared, mutable reference to the current search engine instance.
    /// `None` when no index has been created yet.
    pub engine: Arc<RwLock<Option<Engine>>>,
    /// Filesystem path where the index data is persisted.
    pub data_dir: PathBuf,
}

#[tonic::async_trait]
impl IndexServiceTrait for IndexService {
    /// Creates a new index with the given schema. Fails if an index already exists.
    async fn create_index(
        &self,
        request: Request<CreateIndexRequest>,
    ) -> Result<Response<CreateIndexResponse>, Status> {
        let req = request.into_inner();
        let proto_schema = req
            .schema
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("schema is required"))?;
        let schema = schema_convert::from_proto(proto_schema).map_err(Status::invalid_argument)?;

        let mut guard = self.engine.write().await;
        if guard.is_some() {
            return Err(Status::already_exists("Index already exists"));
        }

        let engine = context::create_index(&self.data_dir, &schema)
            .await
            .map_err(error::anyhow_to_status)?;
        *guard = Some(engine);

        tracing::info!("Index created at {}", self.data_dir.display());
        Ok(Response::new(CreateIndexResponse {}))
    }

    /// Returns index-level statistics such as document count and per-field vector stats.
    async fn get_index(
        &self,
        _request: Request<GetIndexRequest>,
    ) -> Result<Response<GetIndexResponse>, Status> {
        let guard = self.engine.read().await;
        let engine = guard
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("No index is open"))?;

        let stats = engine.stats().map_err(error::to_status)?;

        let vector_fields = stats
            .fields
            .iter()
            .map(|(name, fs)| {
                (
                    name.clone(),
                    VectorFieldStats {
                        vector_count: fs.vector_count as u64,
                        dimension: fs.dimension as u64,
                    },
                )
            })
            .collect();

        Ok(Response::new(GetIndexResponse {
            document_count: stats.document_count as u64,
            vector_fields,
        }))
    }

    /// Returns the schema definition of the current index.
    async fn get_schema(
        &self,
        _request: Request<GetSchemaRequest>,
    ) -> Result<Response<GetSchemaResponse>, Status> {
        let schema = context::read_schema(&self.data_dir).map_err(error::anyhow_to_status)?;
        let proto_schema = schema_convert::to_proto(&schema);
        Ok(Response::new(GetSchemaResponse {
            schema: Some(proto_schema),
        }))
    }
}
