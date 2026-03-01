use std::sync::Arc;

use tokio::sync::RwLock;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use laurus::Engine;

use crate::convert::{error, search as search_convert};
use crate::proto::laurus::v1::{
    search_service_server::SearchService as SearchServiceTrait, SearchRequest, SearchResponse,
    SearchResult,
};

/// gRPC SearchService implementation.
#[derive(Clone)]
pub struct SearchService {
    pub engine: Arc<RwLock<Option<Engine>>>,
}

#[tonic::async_trait]
impl SearchServiceTrait for SearchService {
    async fn search(
        &self,
        request: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        let req = request.into_inner();
        let search_request = search_convert::from_proto(&req)?;

        let guard = self.engine.read().await;
        let engine = guard
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("No index is open"))?;

        let results = engine.search(search_request).await.map_err(error::to_status)?;
        let results: Vec<SearchResult> = results.iter().map(search_convert::result_to_proto).collect();

        Ok(Response::new(SearchResponse { results }))
    }

    type SearchStreamStream = ReceiverStream<Result<SearchResult, Status>>;

    async fn search_stream(
        &self,
        request: Request<SearchRequest>,
    ) -> Result<Response<Self::SearchStreamStream>, Status> {
        let req = request.into_inner();
        let search_request = search_convert::from_proto(&req)?;

        let guard = self.engine.read().await;
        let engine = guard
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("No index is open"))?;

        let results = engine.search(search_request).await.map_err(error::to_status)?;

        let (tx, rx) = tokio::sync::mpsc::channel(64);
        tokio::spawn(async move {
            for result in &results {
                let proto = search_convert::result_to_proto(result);
                if tx.send(Ok(proto)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
