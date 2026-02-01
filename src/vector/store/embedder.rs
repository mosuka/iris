//! VectorStore embedding related type definitions.
//!
//! This module provides the embedding executor for async embedding operations.

use std::future::Future;
use std::sync::{Arc, mpsc};

use tokio::runtime::Builder as TokioRuntimeBuilder;

use crate::error::{IrisError, Result};

/// Executor for running async embedding operations.
#[derive(Clone)]
pub struct EmbedderExecutor {
    runtime: Arc<tokio::runtime::Runtime>,
}

impl EmbedderExecutor {
    /// Create a new embedder executor with a tokio runtime.
    pub(crate) fn new() -> Result<Self> {
        let runtime = TokioRuntimeBuilder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|err| {
                IrisError::internal(format!("failed to initialize embedder runtime: {err}"))
            })?;
        Ok(Self {
            runtime: Arc::new(runtime),
        })
    }

    /// Run an async future and wait for its result.
    pub(crate) fn run<F, T>(&self, future: F) -> Result<T>
    where
        F: Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let handle = self.runtime.handle().clone();
        handle.spawn(async move {
            let _ = tx.send(future.await);
        });
        rx.recv()
            .map_err(|err| IrisError::internal(format!("embedder task channel closed: {err}")))?
    }
}
