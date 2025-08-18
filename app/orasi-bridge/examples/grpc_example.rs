//! Example demonstrating the gRPC server implementation

use bridge_api::{
    config::BridgeAPIConfig,
    services::grpc::{create_grpc_server, GrpcServer},
    metrics::ApiMetrics,
};
use std::time::Duration;
use tonic::{transport::Channel, Request};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create configuration
    let config = BridgeAPIConfig::default();

    // Create metrics
    let metrics = ApiMetrics::new();

    // Create gRPC server
    let grpc_server = create_grpc_server(config, metrics);

    tracing::info!("Starting gRPC server example...");

    // Start the server in a separate task
    let server_handle = tokio::spawn(async move {
        if let Err(e) = grpc_server.start().await {
            tracing::error!("gRPC server error: {}", e);
        }
    });

    // Give the server a moment to start
    tokio::time::sleep(Duration::from_secs(1)).await;

    tracing::info!("gRPC server example completed successfully");

    // Wait for the server to finish (it won't in this example, but in a real app you'd have shutdown logic)
    server_handle.await?;

    Ok(())
}
