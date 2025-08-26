//! gRPC health check service

use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
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

    type WatchStream = tonic::codegen::tokio_stream::wrappers::ReceiverStream<
        std::result::Result<HealthCheckResponse, Status>,
    >;

    async fn watch(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        let health_request = request.into_inner();

        tracing::info!(
            "Health watch request for service: {}",
            health_request.service
        );

        // Validate service name
        if !health_request.service.is_empty() && health_request.service != "bridge-api" {
            return Err(Status::not_found(format!(
                "Service '{}' not found",
                health_request.service
            )));
        }

        let (tx, rx) = mpsc::channel(100); // Buffer size of 100
        let service_name = health_request.service.clone();

        // Spawn health monitoring task
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds
            let mut consecutive_failures = 0;
            const MAX_FAILURES: u32 = 3;

            loop {
                interval.tick().await;

                // Simulate health check logic
                let status = if Self::is_service_healthy(&service_name).await {
                    consecutive_failures = 0;
                    ServingStatus::Serving
                } else {
                    consecutive_failures += 1;
                    if consecutive_failures >= MAX_FAILURES {
                        ServingStatus::NotServing
                    } else {
                        ServingStatus::Serving
                    }
                };

                let response = HealthCheckResponse {
                    status: status.into(),
                };

                tracing::debug!(
                    "Health watch update for service '{}': {:?}",
                    service_name,
                    status
                );

                // Send health status update
                if tx.send(Ok(response)).await.is_err() {
                    tracing::debug!("Health watch client disconnected");
                    break;
                }
            }
        });

        let stream = tonic::codegen::tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Response::new(stream))
    }
}

impl GrpcHealthService {
    /// Check if a service is healthy
    async fn is_service_healthy(service_name: &str) -> bool {
        // For now, we'll consider the bridge-api service as always healthy
        // In a real implementation, this would check actual service health
        if service_name.is_empty() || service_name == "bridge-api" {
            // Simulate occasional health issues (1% chance of failure)
            fastrand::u8(1..101) > 1
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Request;

    #[tokio::test]
    async fn test_health_check() {
        let service = GrpcHealthService::new();
        let request = Request::new(HealthCheckRequest {
            service: "bridge-api".to_string(),
        });

        let response = service.check(request).await.unwrap();
        let health_response = response.into_inner();
        let status = ServingStatus::from_i32(health_response.status).unwrap();

        assert_eq!(status, ServingStatus::Serving);
    }

    #[tokio::test]
    async fn test_health_check_unknown_service() {
        let service = GrpcHealthService::new();
        let request = Request::new(HealthCheckRequest {
            service: "unknown-service".to_string(),
        });

        let response = service.check(request).await.unwrap();
        let health_response = response.into_inner();
        let status = ServingStatus::from_i32(health_response.status).unwrap();

        assert_eq!(status, ServingStatus::ServiceUnknown);
    }

    #[tokio::test]
    async fn test_health_watch_stream_creation() {
        let service = GrpcHealthService::new();
        let request = Request::new(HealthCheckRequest {
            service: "bridge-api".to_string(),
        });

        let response = service.watch(request).await.unwrap();
        let stream = response.into_inner();

        // Verify that the stream was created successfully
        assert!(stream.receiver.try_recv().is_err()); // Should be empty initially
    }

    #[tokio::test]
    async fn test_health_watch_unknown_service() {
        let service = GrpcHealthService::new();
        let request = Request::new(HealthCheckRequest {
            service: "unknown-service".to_string(),
        });

        let result = service.watch(request).await;
        assert!(result.is_err());
        
        if let Err(status) = result {
            assert_eq!(status.code(), tonic::Code::NotFound);
        }
    }

    #[tokio::test]
    async fn test_is_service_healthy() {
        // Test known service
        let result = GrpcHealthService::is_service_healthy("bridge-api").await;
        assert!(result);

        // Test empty service name
        let result = GrpcHealthService::is_service_healthy("").await;
        assert!(result);

        // Test unknown service
        let result = GrpcHealthService::is_service_healthy("unknown-service").await;
        assert!(!result);
    }
}
