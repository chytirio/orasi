//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OTLP receiver for the OpenTelemetry Data Lake Bridge
//!
//! This module provides an OTLP receiver that can ingest OpenTelemetry
//! data via gRPC protocol.

use crate::error::BridgeResult;
use crate::health::checker::HealthCheckCallback;
use crate::health::types::HealthCheckResult;
use crate::traits::TelemetryReceiver;
use crate::types::{
    LogData, LogsBatch, MetricData, MetricType, MetricValue, MetricsBatch, TelemetryBatch,
    TelemetryRecord, TelemetryType, TraceData, TracesBatch,
};
use async_trait::async_trait;
use futures::StreamExt;
use opentelemetry_proto::tonic::collector::logs::v1::logs_service_server::{
    LogsService, LogsServiceServer,
};
use opentelemetry_proto::tonic::collector::logs::v1::{
    ExportLogsServiceRequest, ExportLogsServiceResponse,
};
use opentelemetry_proto::tonic::collector::metrics::v1::metrics_service_server::{
    MetricsService, MetricsServiceServer,
};
use opentelemetry_proto::tonic::collector::metrics::v1::{
    ExportMetricsServiceRequest, ExportMetricsServiceResponse,
};
use opentelemetry_proto::tonic::collector::trace::v1::trace_service_server::{
    TraceService, TraceServiceServer,
};
use opentelemetry_proto::tonic::collector::trace::v1::{
    ExportTraceServiceRequest, ExportTraceServiceResponse,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{debug, error, info, warn};

/// OTLP receiver configuration
#[derive(Debug, Clone)]
pub struct OtlpReceiverConfig {
    /// gRPC endpoint to listen on
    pub endpoint: String,

    /// Maximum concurrent connections
    pub max_concurrent_connections: usize,

    /// Buffer size for incoming data
    pub buffer_size: usize,

    /// Enable TLS
    pub enable_tls: bool,

    /// TLS certificate path (if enabled)
    pub tls_cert_path: Option<String>,

    /// TLS key path (if enabled)
    pub tls_key_path: Option<String>,

    /// Enable compression
    pub enable_compression: bool,

    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
}

impl Default for OtlpReceiverConfig {
    fn default() -> Self {
        Self {
            endpoint: "0.0.0.0:4317".to_string(),
            max_concurrent_connections: 100,
            buffer_size: 10000,
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            enable_compression: true,
            request_timeout_seconds: 30,
        }
    }
}

/// OTLP receiver statistics
#[derive(Debug, Clone)]
pub struct OtlpReceiverStats {
    /// Total records received
    pub total_records: u64,

    /// Records received in last minute
    pub records_per_minute: u64,

    /// Total bytes received
    pub total_bytes: u64,

    /// Bytes received in last minute
    pub bytes_per_minute: u64,

    /// Error count
    pub error_count: u64,

    /// Last receive timestamp
    pub last_receive_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Total requests received
    pub total_requests: u64,

    /// Failed requests
    pub failed_requests: u64,
}

impl Default for OtlpReceiverStats {
    fn default() -> Self {
        Self {
            total_records: 0,
            records_per_minute: 0,
            total_bytes: 0,
            bytes_per_minute: 0,
            error_count: 0,
            last_receive_time: None,
            total_requests: 0,
            failed_requests: 0,
        }
    }
}

/// OTLP receiver implementation
#[derive(Debug, Clone)]
pub struct OtlpReceiver {
    /// Configuration
    pub config: OtlpReceiverConfig,

    /// Statistics
    pub stats: Arc<RwLock<OtlpReceiverStats>>,

    /// Internal buffer for telemetry batches
    pub buffer: Arc<RwLock<Vec<TelemetryBatch>>>,

    /// Running state
    pub running: Arc<RwLock<bool>>,

    /// Server handle for graceful shutdown
    pub server_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl OtlpReceiver {
    /// Create a new OTLP receiver
    pub fn new(config: OtlpReceiverConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(OtlpReceiverStats::default())),
            buffer: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(false)),
            server_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the OTLP receiver
    pub async fn start(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(crate::error::BridgeError::internal(
                "OTLP receiver is already running",
            ));
        }

        *running = true;
        info!("Starting OTLP receiver on {}", self.config.endpoint);

        // Parse the endpoint
        let addr: SocketAddr = self.config.endpoint.parse().map_err(|e| {
            crate::error::BridgeError::configuration(format!("Invalid endpoint: {}", e))
        })?;

        // Create the gRPC service
        let service = OtlpService {
            receiver: Arc::new(self.clone()),
        };

        // Start the gRPC server
        let server_handle = tokio::spawn(async move {
            let server = Server::builder()
                .add_service(TraceServiceServer::new(service.clone()))
                .add_service(MetricsServiceServer::new(service.clone()))
                .add_service(LogsServiceServer::new(service.clone()));

            match server.serve(addr).await {
                Ok(_) => info!("OTLP gRPC server stopped"),
                Err(e) => error!("OTLP gRPC server error: {}", e),
            }
        });

        // Store the server handle
        {
            let mut handle = self.server_handle.write().await;
            *handle = Some(server_handle);
        }

        info!(
            "OTLP receiver started successfully on {}",
            self.config.endpoint
        );
        Ok(())
    }

    /// Stop the OTLP receiver
    pub async fn stop(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "OTLP receiver is not running",
            ));
        }

        *running = false;
        info!("Stopping OTLP receiver");

        // Cancel the server task
        {
            let mut handle = self.server_handle.write().await;
            if let Some(handle) = handle.take() {
                handle.abort();
            }
        }

        info!("OTLP receiver stopped successfully");
        Ok(())
    }
}

/// gRPC service implementation
#[derive(Debug, Clone)]
struct OtlpService {
    receiver: Arc<OtlpReceiver>,
}

#[tonic::async_trait]
impl TraceService for OtlpService {
    async fn export(
        &self,
        request: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, Status> {
        let request_data = request.into_inner();

        // Update statistics
        {
            let mut stats = self.receiver.stats.write().await;
            stats.total_requests += 1;
            stats.last_receive_time = Some(chrono::Utc::now());
        }

        // For now, just create a simple mock record
        let mock_record = TelemetryRecord::new(
            TelemetryType::Trace,
            crate::types::TelemetryData::Trace(TraceData {
                trace_id: "mock_trace_id".to_string(),
                span_id: "mock_span_id".to_string(),
                parent_span_id: None,
                name: "mock_span".to_string(),
                kind: crate::types::traces::SpanKind::Internal,
                start_time: chrono::Utc::now(),
                end_time: None,
                duration_ns: None,
                status: crate::types::traces::SpanStatus {
                    code: crate::types::traces::StatusCode::Ok,
                    message: None,
                },
                attributes: HashMap::new(),
                events: Vec::new(),
                links: Vec::new(),
            }),
        );

        let batch = TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: "otlp_traces".to_string(),
            size: 1,
            records: vec![mock_record],
            metadata: HashMap::new(),
        };

        // Add to buffer
        {
            let mut buffer = self.receiver.buffer.write().await;
            buffer.push(batch);
        }

        // Update statistics
        {
            let mut stats = self.receiver.stats.write().await;
            stats.total_records += 1;
        }

        Ok(Response::new(ExportTraceServiceResponse {
            partial_success: None,
        }))
    }
}

#[tonic::async_trait]
impl MetricsService for OtlpService {
    async fn export(
        &self,
        request: Request<ExportMetricsServiceRequest>,
    ) -> Result<Response<ExportMetricsServiceResponse>, Status> {
        let request_data = request.into_inner();

        // Update statistics
        {
            let mut stats = self.receiver.stats.write().await;
            stats.total_requests += 1;
            stats.last_receive_time = Some(chrono::Utc::now());
        }

        // For now, just create a simple mock record
        let mock_record = TelemetryRecord::new(
            TelemetryType::Metric,
            crate::types::TelemetryData::Metric(MetricData {
                name: "mock_metric".to_string(),
                description: Some("Mock metric from OTLP".to_string()),
                unit: Some("count".to_string()),
                metric_type: MetricType::Counter,
                value: MetricValue::Counter(1.0),
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            }),
        );

        let batch = TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: "otlp_metrics".to_string(),
            size: 1,
            records: vec![mock_record],
            metadata: HashMap::new(),
        };

        // Add to buffer
        {
            let mut buffer = self.receiver.buffer.write().await;
            buffer.push(batch);
        }

        // Update statistics
        {
            let mut stats = self.receiver.stats.write().await;
            stats.total_records += 1;
        }

        Ok(Response::new(ExportMetricsServiceResponse {
            partial_success: None,
        }))
    }
}

#[tonic::async_trait]
impl LogsService for OtlpService {
    async fn export(
        &self,
        request: Request<ExportLogsServiceRequest>,
    ) -> Result<Response<ExportLogsServiceResponse>, Status> {
        let request_data = request.into_inner();

        // Update statistics
        {
            let mut stats = self.receiver.stats.write().await;
            stats.total_requests += 1;
            stats.last_receive_time = Some(chrono::Utc::now());
        }

        // For now, just create a simple mock record
        let mock_record = TelemetryRecord::new(
            TelemetryType::Log,
            crate::types::TelemetryData::Log(LogData {
                timestamp: chrono::Utc::now(),
                level: crate::types::logs::LogLevel::Info,
                message: "Mock log from OTLP".to_string(),
                attributes: HashMap::new(),
                body: Some("Mock log body".to_string()),
                severity_number: Some(9), // INFO level
                severity_text: Some("INFO".to_string()),
            }),
        );

        let batch = TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: "otlp_logs".to_string(),
            size: 1,
            records: vec![mock_record],
            metadata: HashMap::new(),
        };

        // Add to buffer
        {
            let mut buffer = self.receiver.buffer.write().await;
            buffer.push(batch);
        }

        // Update statistics
        {
            let mut stats = self.receiver.stats.write().await;
            stats.total_records += 1;
        }

        Ok(Response::new(ExportLogsServiceResponse {
            partial_success: None,
        }))
    }
}

#[async_trait]
impl TelemetryReceiver for OtlpReceiver {
    async fn receive(&self) -> BridgeResult<TelemetryBatch> {
        let mut buffer = self.buffer.write().await;
        if let Some(batch) = buffer.pop() {
            Ok(batch)
        } else {
            // Return empty batch if no data available
            Ok(TelemetryBatch {
                id: uuid::Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
                source: "otlp".to_string(),
                size: 0,
                records: vec![],
                metadata: HashMap::new(),
            })
        }
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        let running = self.running.read().await;
        Ok(*running)
    }

    async fn get_stats(&self) -> BridgeResult<crate::traits::receiver::ReceiverStats> {
        let stats = self.stats.read().await;
        Ok(crate::traits::receiver::ReceiverStats {
            total_records: stats.total_records,
            records_per_minute: stats.records_per_minute,
            total_bytes: stats.total_bytes,
            bytes_per_minute: stats.bytes_per_minute,
            error_count: stats.error_count,
            last_receive_time: stats.last_receive_time,
            protocol_stats: Some(HashMap::from([
                (
                    "total_requests".to_string(),
                    stats.total_requests.to_string(),
                ),
                (
                    "failed_requests".to_string(),
                    stats.failed_requests.to_string(),
                ),
            ])),
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        self.stop().await
    }
}

#[async_trait]
impl HealthCheckCallback for OtlpReceiver {
    async fn check(&self) -> BridgeResult<HealthCheckResult> {
        let running = self.running.read().await;
        let status = if *running {
            crate::health::types::HealthStatus::Healthy
        } else {
            crate::health::types::HealthStatus::Unhealthy
        };

        Ok(HealthCheckResult::new(
            "otlp_receiver".to_string(),
            status,
            if *running {
                "OTLP receiver is running".to_string()
            } else {
                "OTLP receiver is not running".to_string()
            },
        ))
    }

    fn component_name(&self) -> &str {
        "otlp_receiver"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_otlp_receiver_start_stop() {
        let config = OtlpReceiverConfig {
            endpoint: "127.0.0.1:0".to_string(), // Use port 0 for dynamic assignment
            ..Default::default()
        };

        let receiver = OtlpReceiver::new(config);

        // Test start
        assert!(receiver.start().await.is_ok());

        // Test that it's running
        let running = receiver.running.read().await;
        assert!(*running);
        drop(running);

        // Test stop
        assert!(receiver.stop().await.is_ok());

        // Test that it's stopped
        let running = receiver.running.read().await;
        assert!(!*running);
    }

    #[tokio::test]
    async fn test_otlp_receiver_double_start() {
        let config = OtlpReceiverConfig {
            endpoint: "127.0.0.1:0".to_string(),
            ..Default::default()
        };

        let receiver = OtlpReceiver::new(config);

        // First start should succeed
        assert!(receiver.start().await.is_ok());

        // Second start should fail
        assert!(receiver.start().await.is_err());

        // Clean up
        assert!(receiver.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_otlp_receiver_stop_when_not_running() {
        let config = OtlpReceiverConfig {
            endpoint: "127.0.0.1:0".to_string(),
            ..Default::default()
        };

        let receiver = OtlpReceiver::new(config);

        // Stop when not running should fail
        assert!(receiver.stop().await.is_err());
    }
}
