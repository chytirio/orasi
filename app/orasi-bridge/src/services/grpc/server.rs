//! gRPC server management

use tonic::transport::Server;
use tracing;

use crate::{
    config::BridgeAPIConfig,
    error::ApiError,
    metrics::ApiMetrics,
    proto::*,
    services::grpc::{bridge::BridgeGrpcService, health::GrpcHealthService},
};

/// gRPC server for Bridge API
pub struct GrpcServer {
    config: BridgeAPIConfig,
    metrics: ApiMetrics,
}

impl GrpcServer {
    /// Create a new gRPC server
    pub fn new(config: BridgeAPIConfig, metrics: ApiMetrics) -> Self {
        Self { config, metrics }
    }

    /// Start the gRPC server
    pub async fn start(self) -> Result<(), ApiError> {
        let addr = self
            .config
            .grpc_address()
            .parse::<std::net::SocketAddr>()
            .map_err(|e| ApiError::Internal(format!("Invalid gRPC address: {}", e)))?;

        tracing::info!("Starting gRPC server on {}", self.config.grpc_address());

        // Create services
        let mut bridge_service = BridgeGrpcService::new(self.config.clone(), self.metrics.clone());
        let health_service = GrpcHealthService::new();

        // Initialize the bridge service
        bridge_service
            .init()
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to initialize gRPC service: {}", e)))?;

        // Create server with services
        let server = Server::builder()
            .add_service(bridge_service_server::BridgeServiceServer::new(
                bridge_service,
            ))
            .add_service(health_server::HealthServer::new(health_service));

        tracing::info!(
            "gRPC server starting on {} with bridge and health services",
            addr
        );

        // Start the server
        server
            .serve(addr)
            .await
            .map_err(|e| ApiError::Internal(format!("gRPC server error: {}", e)))?;

        Ok(())
    }
}

/// Create gRPC server with all services
pub fn create_grpc_server(config: BridgeAPIConfig, metrics: ApiMetrics) -> GrpcServer {
    GrpcServer::new(config, metrics)
}

/// Create gRPC server with only OpenTelemetry services
pub fn create_otlp_grpc_server(config: BridgeAPIConfig, metrics: ApiMetrics) -> GrpcServer {
    GrpcServer::new(config, metrics)
}

/// Create gRPC server with only custom bridge services
pub fn create_bridge_grpc_server(config: BridgeAPIConfig, metrics: ApiMetrics) -> GrpcServer {
    GrpcServer::new(config, metrics)
}

/// Create minimal gRPC server (health checks only)
pub fn create_minimal_grpc_server(config: BridgeAPIConfig, metrics: ApiMetrics) -> GrpcServer {
    GrpcServer::new(config, metrics)
}
