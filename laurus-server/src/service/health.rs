//! Health-check gRPC service.
//!
//! Provides a simple liveness probe that always returns `SERVING`.

use tonic::{Request, Response, Status};

use crate::proto::laurus::v1::{
    self, HealthCheckRequest, HealthCheckResponse,
    health_service_server::HealthService as HealthServiceTrait,
};

/// gRPC HealthService implementation.
#[derive(Debug)]
pub struct HealthService;

#[tonic::async_trait]
impl HealthServiceTrait for HealthService {
    /// Performs a health check and always returns `SERVING` status.
    async fn check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            status: v1::ServingStatus::Serving as i32,
        }))
    }
}
