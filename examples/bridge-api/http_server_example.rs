//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating HTTP server for OTLP ingestion
//!
//! This example shows how to create an HTTP server that receives
//! OpenTelemetry data over HTTP and integrates with the bridge-core pipeline.

use async_trait::async_trait;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use bridge_core::{
    pipeline::PipelineConfig,
    traits::ExporterStats,
    types::{
        ExportResult, ExportStatus, MetricData, MetricValue, ProcessedBatch, ProcessingStatus,
        TelemetryData, TelemetryRecord, TelemetryType,
    },
    BridgeResult, LakehouseExporter, TelemetryIngestionPipeline, TelemetryReceiver,
};
use uuid::Uuid;

/// HTTP server state
#[derive(Clone)]
struct HttpServerState {
    pipeline: Arc<RwLock<TelemetryIngestionPipeline>>,
    request_count: Arc<RwLock<u64>>,
}

/// HTTP receiver that receives OTLP data over HTTP
pub struct HttpOtlpReceiver {
    name: String,
    is_running: bool,
    received_data: Arc<RwLock<Vec<Vec<u8>>>>,
    request_count: Arc<RwLock<u64>>,
}

impl HttpOtlpReceiver {
    pub fn new(name: String) -> Self {
        Self {
            name,
            is_running: false,
            received_data: Arc::new(RwLock::new(Vec::new())),
            request_count: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn add_received_data(&self, data: Vec<u8>) {
        let mut received_data = self.received_data.write().await;
        received_data.push(data);

        let mut request_count = self.request_count.write().await;
        *request_count += 1;

        info!(
            "HTTP OTLP receiver '{}' received data, total requests: {}",
            self.name, *request_count
        );
    }

    pub async fn get_received_data_count(&self) -> usize {
        let received_data = self.received_data.read().await;
        received_data.len()
    }

    pub async fn get_request_count(&self) -> u64 {
        let request_count = self.request_count.read().await;
        *request_count
    }
}

#[async_trait]
impl TelemetryReceiver for HttpOtlpReceiver {
    async fn receive(&self) -> BridgeResult<bridge_core::types::TelemetryBatch> {
        // Get the most recent received data
        let received_data = self.received_data.read().await;

        if received_data.is_empty() {
            // Return empty batch if no data received
            let empty_batch = bridge_core::types::TelemetryBatch {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                source: "http-otlp".to_string(),
                size: 0,
                records: Vec::new(),
                metadata: HashMap::new(),
            };
            return Ok(empty_batch);
        }

        // Convert received data to telemetry records
        let mut records = Vec::new();

        for (i, data) in received_data.iter().enumerate() {
            // Parse the received data as JSON (simplified for example)
            // In a real implementation, this would parse OTLP protobuf or JSON format

            let record = TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name: format!("http_otlp_metric_{}", i),
                    description: Some(format!("HTTP OTLP metric {} from HTTP ingestion", i)),
                    unit: Some("count".to_string()),
                    metric_type: bridge_core::types::MetricType::Gauge,
                    value: MetricValue::Gauge(data.len() as f64), // Use data length as metric value
                    labels: HashMap::new(),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::new(),
                tags: HashMap::new(),
                resource: None,
                service: None,
            };
            records.push(record);
        }

        let batch = bridge_core::types::TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "http-otlp".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::new(),
        };

        info!(
            "HTTP OTLP receiver '{}' generated batch with {} records from {} received requests",
            self.name,
            batch.size,
            received_data.len()
        );
        Ok(batch)
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        Ok(self.is_running)
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ReceiverStats> {
        let request_count = self.get_request_count().await;
        let received_data_count = self.get_received_data_count().await;

        Ok(bridge_core::traits::ReceiverStats {
            total_records: request_count,
            records_per_minute: request_count / 60, // Simplified calculation
            total_bytes: (received_data_count * 100) as u64, // Estimate
            bytes_per_minute: (received_data_count * 100) as u64 / 60,
            error_count: 0,
            last_receive_time: Some(Utc::now()),
            protocol_stats: Some(HashMap::from([
                ("protocol".to_string(), "http".to_string()),
                ("endpoint".to_string(), "/v1/traces".to_string()),
            ])),
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("HTTP OTLP receiver '{}' shutting down", self.name);
        Ok(())
    }
}

/// HTTP-aware exporter that handles HTTP-received data
pub struct HttpOtlpExporter {
    name: String,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ExporterStats>>,
}

impl HttpOtlpExporter {
    pub fn new(name: String) -> Self {
        let stats = ExporterStats {
            total_batches: 0,
            total_records: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            avg_export_time_ms: 0.0,
            error_count: 0,
            last_export_time: None,
        };

        Self {
            name,
            is_running: Arc::new(RwLock::new(true)), // Start as running
            stats: Arc::new(RwLock::new(stats)),
        }
    }

    async fn update_stats(&self, records: usize, duration: Duration, success: bool) {
        let mut stats = self.stats.write().await;

        stats.total_batches += 1;
        stats.total_records += records as u64;
        stats.last_export_time = Some(Utc::now());

        if success {
            let duration_ms = duration.as_millis() as f64;
            if stats.total_batches > 1 {
                stats.avg_export_time_ms =
                    (stats.avg_export_time_ms * (stats.total_batches - 1) as f64 + duration_ms)
                        / stats.total_batches as f64;
            } else {
                stats.avg_export_time_ms = duration_ms;
            }
        } else {
            stats.error_count += 1;
        }
    }

    async fn http_export(&self, batch: &ProcessedBatch) -> (bool, usize, Vec<String>) {
        // Simulate HTTP export operation
        // In a real implementation, this would:
        // 1. Check if data is from HTTP ingestion
        // 2. Apply HTTP-specific export logic
        // 3. Handle HTTP data sinks
        // 4. Return export results

        let records_count = batch.records.len();

        // Check if this is HTTP-received data
        let is_http_data = batch
            .metadata
            .get("source")
            .map(|s| s == "http-otlp")
            .unwrap_or(false);

        if is_http_data {
            info!(
                "HTTP OTLP exporter '{}' exporting HTTP-received data: {} records",
                self.name, records_count
            );

            // Simulate success for HTTP data
            (true, records_count, Vec::new())
        } else {
            // Simulate failure for non-HTTP data
            (false, 0, vec!["Data not from HTTP ingestion".to_string()])
        }
    }
}

#[async_trait]
impl LakehouseExporter for HttpOtlpExporter {
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        let start_time = Instant::now();

        // Check if exporter is running
        if !*self.is_running.read().await {
            return Err(bridge_core::error::BridgeError::export(
                "HTTP OTLP exporter is not running",
            ));
        }

        info!(
            "HTTP OTLP exporter '{}' exporting batch with {} records",
            self.name,
            batch.records.len()
        );

        // HTTP export operation
        let (success, records_written, errors) = self.http_export(&batch).await;

        let duration = start_time.elapsed();

        // Update statistics
        self.update_stats(batch.records.len(), duration, success)
            .await;

        // Convert errors to ExportError format
        let export_errors: Vec<bridge_core::types::ExportError> = errors
            .iter()
            .map(|e| bridge_core::types::ExportError {
                code: "HTTP_OTLP_EXPORT_ERROR".to_string(),
                message: e.clone(),
                details: None,
            })
            .collect();

        // Create export result
        let export_result = ExportResult {
            timestamp: Utc::now(),
            status: if success {
                ExportStatus::Success
            } else {
                ExportStatus::Failed
            },
            records_exported: records_written,
            records_failed: batch.records.len() - records_written,
            duration_ms: duration.as_millis() as u64,
            metadata: HashMap::new(),
            errors: export_errors,
        };

        if success {
            info!(
                "HTTP OTLP exporter '{}' successfully exported {} records in {:?}",
                self.name, records_written, duration
            );
        } else {
            warn!(
                "HTTP OTLP exporter '{}' failed to export some records: {} errors",
                self.name,
                errors.len()
            );
        }

        Ok(export_result)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        Ok(*self.is_running.read().await)
    }

    async fn get_stats(&self) -> BridgeResult<ExporterStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("HTTP OTLP exporter '{}' shutting down", self.name);

        let mut is_running = self.is_running.write().await;
        *is_running = false;

        info!("HTTP OTLP exporter '{}' shutdown completed", self.name);
        Ok(())
    }
}

/// HTTP handlers
async fn health_check() -> StatusCode {
    StatusCode::OK
}

async fn receive_otlp_data(
    State(state): State<HttpServerState>,
    Json(payload): Json<Value>,
) -> StatusCode {
    let start_time = Instant::now();

    info!(
        "Received OTLP data via HTTP: {} bytes",
        serde_json::to_string(&payload).unwrap().len()
    );

    // Convert JSON payload to bytes for storage
    let data = serde_json::to_vec(&payload).unwrap_or_default();

    // In a real implementation, we would add this data to the receiver
    // For now, we just log that we received the data
    info!("Received OTLP data: {} bytes", data.len());

    // Update request count
    {
        let mut request_count = state.request_count.write().await;
        *request_count += 1;
    }

    let duration = start_time.elapsed();
    info!("Processed HTTP OTLP request in {:?}", duration);

    StatusCode::OK
}

async fn get_stats(State(state): State<HttpServerState>) -> Json<Value> {
    let request_count = *state.request_count.read().await;

    let stats = serde_json::json!({
        "server": "http-otlp-server",
        "request_count": request_count,
        "status": "running",
        "timestamp": Utc::now().to_rfc3339()
    });

    Json(stats)
}

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting HTTP server example for OTLP ingestion");

    // Create pipeline configuration
    let config = PipelineConfig {
        name: "http-otlp-pipeline".to_string(),
        max_batch_size: 1000,
        flush_interval_ms: 5000,
        buffer_size: 10000,
        enable_backpressure: true,
        backpressure_threshold: 80,
        enable_metrics: true,
        enable_health_checks: true,
        health_check_interval_ms: 30000,
    };

    // Create pipeline
    let mut pipeline = TelemetryIngestionPipeline::new(config);

    // Create and add HTTP receiver
    let mut receiver = HttpOtlpReceiver::new("http-otlp-receiver".to_string());
    receiver.is_running = true; // Make receiver healthy
    let receiver = Arc::new(receiver);
    pipeline.add_receiver(receiver);

    // Create and add HTTP exporter
    let http_exporter = HttpOtlpExporter::new("http-otlp-exporter".to_string());
    let http_exporter = Arc::new(http_exporter);
    pipeline.add_exporter(http_exporter);

    // Start pipeline
    pipeline.start().await?;

    info!("HTTP OTLP pipeline started successfully");

    // Create HTTP server state
    let state = HttpServerState {
        pipeline: Arc::new(RwLock::new(pipeline)),
        request_count: Arc::new(RwLock::new(0)),
    };

    // Create HTTP router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/v1/traces", post(receive_otlp_data))
        .route("/v1/metrics", post(receive_otlp_data))
        .route("/v1/logs", post(receive_otlp_data))
        .route("/stats", get(get_stats))
        .with_state(state);

    // Start HTTP server
    let server_addr = "127.0.0.1:4318";
    info!("Starting HTTP server on {}", server_addr);

    let listener = tokio::net::TcpListener::bind(server_addr).await?;
    info!("HTTP server listening on {}", server_addr);

    // Run the server for a few seconds
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service()).await.unwrap();
    });

    // Let the server run for a few seconds
    tokio::time::sleep(Duration::from_secs(10)).await;

    info!("HTTP server example completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_otlp_receiver_creation() {
        let receiver = HttpOtlpReceiver::new("test-receiver".to_string());
        assert_eq!(receiver.get_request_count().await, 0);
        assert_eq!(receiver.get_received_data_count().await, 0);
    }

    #[tokio::test]
    async fn test_http_otlp_receiver_data_reception() {
        let receiver = HttpOtlpReceiver::new("test-receiver".to_string());

        // Add some test data
        receiver.add_received_data(vec![1, 2, 3, 4, 5]).await;
        receiver.add_received_data(vec![6, 7, 8, 9, 10]).await;

        assert_eq!(receiver.get_request_count().await, 2);
        assert_eq!(receiver.get_received_data_count().await, 2);
    }

    #[tokio::test]
    async fn test_http_otlp_exporter_creation() {
        let exporter = HttpOtlpExporter::new("test-exporter".to_string());
        assert_eq!(exporter.name(), "test-exporter");
        assert_eq!(exporter.version(), "1.0.0");
    }

    #[tokio::test]
    async fn test_http_otlp_exporter_export() {
        let exporter = HttpOtlpExporter::new("test-exporter".to_string());

        // Create a test batch with HTTP source metadata
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "test_metric".to_string(),
                description: Some("Test metric".to_string()),
                unit: Some("count".to_string()),
                metric_type: bridge_core::types::MetricType::Gauge,
                value: MetricValue::Gauge(1.0),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        let processed_record = bridge_core::types::ProcessedRecord {
            original_id: Uuid::new_v4(),
            status: bridge_core::types::ProcessingStatus::Success,
            transformed_data: Some(record.data.clone()),
            metadata: HashMap::new(),
            errors: Vec::new(),
        };

        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), "http-otlp".to_string());

        let processed_batch = ProcessedBatch {
            original_batch_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            records: vec![processed_record],
            metadata,
            status: ProcessingStatus::Success,
            errors: Vec::new(),
        };

        // Export the batch
        let result = exporter.export(processed_batch).await.unwrap();
        assert_eq!(result.status, ExportStatus::Success);
        assert_eq!(result.records_exported, 1);
        assert_eq!(result.records_failed, 0);
    }
}
