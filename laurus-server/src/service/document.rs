use std::sync::Arc;

use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

use laurus::Engine;

use crate::convert::{document as doc_convert, error};
use crate::proto::laurus::v1::{
    AddDocumentRequest, AddDocumentResponse, CommitRequest, CommitResponse, DeleteDocumentsRequest,
    DeleteDocumentsResponse, GetDocumentsRequest, GetDocumentsResponse, PutDocumentRequest,
    PutDocumentResponse, document_service_server::DocumentService as DocumentServiceTrait,
};

/// gRPC DocumentService implementation.
#[derive(Clone)]
pub struct DocumentService {
    pub engine: Arc<RwLock<Option<Engine>>>,
}

impl DocumentService {
    #[allow(clippy::result_large_err)]
    fn get_engine_ref(guard: &Option<Engine>) -> Result<&Engine, Status> {
        guard
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("No index is open. Create an index first."))
    }
}

#[tonic::async_trait]
impl DocumentServiceTrait for DocumentService {
    async fn put_document(
        &self,
        request: Request<PutDocumentRequest>,
    ) -> Result<Response<PutDocumentResponse>, Status> {
        let req = request.into_inner();
        let doc = req
            .document
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("document is required"))?;
        let doc = doc_convert::from_proto(doc);

        let guard = self.engine.read().await;
        let engine = Self::get_engine_ref(&guard)?;
        engine
            .put_document(&req.id, doc)
            .await
            .map_err(error::to_status)?;

        Ok(Response::new(PutDocumentResponse {}))
    }

    async fn add_document(
        &self,
        request: Request<AddDocumentRequest>,
    ) -> Result<Response<AddDocumentResponse>, Status> {
        let req = request.into_inner();
        let doc = req
            .document
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("document is required"))?;
        let doc = doc_convert::from_proto(doc);

        let guard = self.engine.read().await;
        let engine = Self::get_engine_ref(&guard)?;
        engine
            .add_document(&req.id, doc)
            .await
            .map_err(error::to_status)?;

        Ok(Response::new(AddDocumentResponse {}))
    }

    async fn get_documents(
        &self,
        request: Request<GetDocumentsRequest>,
    ) -> Result<Response<GetDocumentsResponse>, Status> {
        let req = request.into_inner();

        let guard = self.engine.read().await;
        let engine = Self::get_engine_ref(&guard)?;
        let docs = engine
            .get_documents(&req.id)
            .await
            .map_err(error::to_status)?;

        let documents = docs.iter().map(doc_convert::to_proto).collect();
        Ok(Response::new(GetDocumentsResponse { documents }))
    }

    async fn delete_documents(
        &self,
        request: Request<DeleteDocumentsRequest>,
    ) -> Result<Response<DeleteDocumentsResponse>, Status> {
        let req = request.into_inner();

        let guard = self.engine.read().await;
        let engine = Self::get_engine_ref(&guard)?;
        engine
            .delete_documents(&req.id)
            .await
            .map_err(error::to_status)?;

        Ok(Response::new(DeleteDocumentsResponse {}))
    }

    async fn commit(
        &self,
        _request: Request<CommitRequest>,
    ) -> Result<Response<CommitResponse>, Status> {
        let guard = self.engine.read().await;
        let engine = Self::get_engine_ref(&guard)?;
        engine.commit().await.map_err(error::to_status)?;

        Ok(Response::new(CommitResponse {}))
    }
}
