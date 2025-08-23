//! gRPC health check service

use std::time::Instant;
use tonic::{Request, Response, Status};
use tracing;

use crate::proto::{
    health_check_response::ServingStatus, health_server, HealthCheckRequest, HealthCheckResponse,
};

/// gRPC health check service
pub struct GrpcHealthService {
    start_time: Instant,
}

impl GrpcHealthService {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
}

#[tonic::async_trait]
impl health_server::Health for GrpcHealthService {
    async fn check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let health_request = request.into_inner();

        tracing::debug!(
            "Health check request for service: {}",
            health_request.service
        );

        // Check if the requested service is healthy
        let status = if health_request.service.is_empty() || health_request.service == "bridge-api"
        {
            ServingStatus::Serving
        } else {
            ServingStatus::ServiceUnknown
        };

        let response = HealthCheckResponse {
            status: status.into(),
        };

        tracing::debug!("Health check response: {:?}", status);

        Ok(Response::new(response))
    }

    // TODO: Implement health watch streaming
    // Temporarily disabled until streaming is properly implemented
}
