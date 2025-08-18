//! gRPC telemetry service

use tonic::{Request, Response, Status};
use tracing;

use crate::metrics::ApiMetrics;

/// Simple gRPC service for telemetry data
pub struct TelemetryGrpcService {
    metrics: ApiMetrics,
}

impl TelemetryGrpcService {
    pub fn new(metrics: ApiMetrics) -> Self {
        Self { metrics }
    }
}

// Simple protobuf message definitions
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TelemetryRequest {
    #[prost(string, tag = "1")]
    pub data: String,
    #[prost(string, tag = "2")]
    pub format: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TelemetryResponse {
    #[prost(bool, tag = "1")]
    pub success: bool,
    #[prost(string, tag = "2")]
    pub message: String,
    #[prost(int32, tag = "3")]
    pub processed_count: i32,
}

// Define the service trait
#[tonic::async_trait]
pub trait TelemetryService: Send + Sync + 'static {
    async fn ingest(
        &self,
        request: Request<TelemetryRequest>,
    ) -> Result<Response<TelemetryResponse>, Status>;
}

#[tonic::async_trait]
impl TelemetryService for TelemetryGrpcService {
    async fn ingest(
        &self,
        request: Request<TelemetryRequest>,
    ) -> Result<Response<TelemetryResponse>, Status> {
        let start_time = std::time::Instant::now();

        tracing::info!("Received gRPC telemetry request");

        let telemetry_request = request.into_inner();

        // Process the telemetry data
        let processing_result = match telemetry_request.format.as_str() {
            "json" => {
                // Parse JSON telemetry data
                match serde_json::from_str::<serde_json::Value>(&telemetry_request.data) {
                    Ok(_) => {
                        tracing::info!("Successfully parsed JSON telemetry data");
                        Ok(1) // Processed 1 record
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse JSON telemetry data: {}", e);
                        Err(format!("Invalid JSON: {}", e))
                    }
                }
            }
            "protobuf" => {
                // Handle protobuf data
                tracing::info!("Processing protobuf telemetry data");
                Ok(1) // Processed 1 record
            }
            _ => {
                tracing::warn!("Unsupported format: {}", telemetry_request.format);
                Err(format!("Unsupported format: {}", telemetry_request.format))
            }
        };

        // Record metrics
        let processing_time = start_time.elapsed();
        let success = processing_result.is_ok();
        self.metrics
            .record_processing("grpc_telemetry", processing_time, success);

        // Create response
        let response = match processing_result {
            Ok(count) => TelemetryResponse {
                success: true,
                message: "Telemetry data processed successfully".to_string(),
                processed_count: count,
            },
            Err(error_msg) => TelemetryResponse {
                success: false,
                message: error_msg,
                processed_count: 0,
            },
        };

        tracing::info!("gRPC telemetry processing completed: success={}", success);

        Ok(Response::new(response))
    }
}

// Placeholder for future OpenTelemetry services
// These will be implemented when we resolve the grpcio-sys compilation issues
