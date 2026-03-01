use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::RwLock;
use tonic::transport::Server;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::context;
use crate::proto::laurus::v1::{
    document_service_server::DocumentServiceServer,
    health_service_server::HealthServiceServer,
    index_service_server::IndexServiceServer,
    search_service_server::SearchServiceServer,
};
use crate::service::{document::DocumentService, health::HealthService, index::IndexService, search::SearchService};

/// Run the gRPC server with the given configuration.
pub async fn run(config: &Config) -> anyhow::Result<()> {
    // Initialize tracing.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&config.log.level)),
        )
        .init();

    tracing::info!("Laurus server starting");
    tracing::info!("Data directory: {}", config.index.data_dir.display());

    // Try to open an existing index, or start without one.
    let engine = match context::open_index(&config.index.data_dir).await {
        Ok(engine) => {
            tracing::info!("Opened existing index");
            Some(engine)
        }
        Err(_) => {
            tracing::info!("No existing index found. Use CreateIndex RPC to create one.");
            None
        }
    };

    let engine = Arc::new(RwLock::new(engine));
    let data_dir = config.index.data_dir.clone();

    let health_service = HealthService;
    let document_service = DocumentService {
        engine: engine.clone(),
    };
    let index_service = IndexService {
        engine: engine.clone(),
        data_dir,
    };
    let search_service = SearchService {
        engine: engine.clone(),
    };

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    tracing::info!("Listening on {addr}");

    Server::builder()
        .add_service(HealthServiceServer::new(health_service))
        .add_service(DocumentServiceServer::new(document_service))
        .add_service(IndexServiceServer::new(index_service))
        .add_service(SearchServiceServer::new(search_service))
        .serve_with_shutdown(addr, shutdown_signal(engine.clone()))
        .await?;

    Ok(())
}

/// Wait for a shutdown signal (Ctrl+C) and commit before exiting.
async fn shutdown_signal(engine: Arc<RwLock<Option<laurus::Engine>>>) {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");

    tracing::info!("Shutdown signal received, committing pending changes...");

    let guard = engine.read().await;
    if let Some(engine) = guard.as_ref() {
        if let Err(e) = engine.commit().await {
            tracing::error!("Failed to commit on shutdown: {e}");
        } else {
            tracing::info!("Committed successfully");
        }
    }

    tracing::info!("Server shutting down");
}
