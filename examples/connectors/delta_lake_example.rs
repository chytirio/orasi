//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating Delta Lake exporter with bridge-core pipeline
//!
//! This example shows how to create a Delta Lake exporter that implements
//! the LakehouseExporter trait and integrates with the telemetry ingestion pipeline.

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn};

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

/// Simple mock receiver for testing
pub struct SimpleMockReceiver {
    name: String,
    is_running: bool,
}

impl SimpleMockReceiver {
    pub fn new(name: String) -> Self {
        Self {
            name,
            is_running: false,
        }
    }
}

#[async_trait]
impl TelemetryReceiver for SimpleMockReceiver {
    async fn receive(&self) -> BridgeResult<bridge_core::types::TelemetryBatch> {
        // Create a simple telemetry record
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "delta_lake_test_metric".to_string(),
                description: Some("Test metric for Delta Lake export".to_string()),
                unit: Some("count".to_string()),
                metric_type: bridge_core::types::MetricType::Gauge,
                value: MetricValue::Gauge(42.0),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        let batch = bridge_core::types::TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "delta-lake-test".to_string(),
            size: 1,
            records: vec![record],
            metadata: HashMap::new(),
        };

        info!(
            "Simple mock receiver '{}' generated batch with {} records",
            self.name, batch.size
        );
        Ok(batch)
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        Ok(self.is_running)
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ReceiverStats> {
        Ok(bridge_core::traits::ReceiverStats {
            total_records: 1,
            records_per_minute: 1,
            total_bytes: 100,
            bytes_per_minute: 100,
            error_count: 0,
            last_receive_time: Some(Utc::now()),
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Simple mock receiver '{}' shutting down", self.name);
        Ok(())
    }
}

/// Delta Lake exporter implementation
pub struct DeltaLakeExporter {
    name: String,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ExporterStats>>,
}

impl DeltaLakeExporter {
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

    pub async fn start(&mut self) {
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        info!("Delta Lake exporter '{}' started", self.name);
    }

    pub async fn stop(&mut self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        info!("Delta Lake exporter '{}' stopped", self.name);
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

    async fn mock_delta_lake_write(&self, batch: &ProcessedBatch) -> (bool, usize, Vec<String>) {
        // Simulate Delta Lake write operation
        // In a real implementation, this would:
        // 1. Convert ProcessedBatch to Delta Lake format
        // 2. Write to Delta Lake table
        // 3. Handle transactions and commits
        // 4. Return actual results

        let records_count = batch.records.len();

        // Simulate some failures for testing
        if records_count > 1000 {
            // Simulate failure for large batches
            (false, 0, vec!["Batch too large for Delta Lake".to_string()])
        } else {
            // Simulate success
            (true, records_count, Vec::new())
        }
    }
}

#[async_trait]
impl LakehouseExporter for DeltaLakeExporter {
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        let start_time = Instant::now();

        // Check if exporter is running
        if !*self.is_running.read().await {
            return Err(bridge_core::error::BridgeError::export(
                "Delta Lake exporter is not running",
            ));
        }

        info!(
            "Delta Lake exporter '{}' exporting batch with {} records",
            self.name,
            batch.records.len()
        );

        // Mock Delta Lake write operation
        let (success, records_written, errors) = self.mock_delta_lake_write(&batch).await;

        let duration = start_time.elapsed();

        // Update statistics
        self.update_stats(batch.records.len(), duration, success)
            .await;

        // Convert errors to ExportError format
        let export_errors: Vec<bridge_core::types::ExportError> = errors
            .iter()
            .map(|e| bridge_core::types::ExportError {
                code: "DELTA_LAKE_ERROR".to_string(),
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
                "Delta Lake exporter '{}' successfully exported {} records in {:?}",
                self.name, records_written, duration
            );
        } else {
            warn!(
                "Delta Lake exporter '{}' failed to export some records: {} errors",
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
        info!("Delta Lake exporter '{}' shutting down", self.name);

        let mut is_running = self.is_running.write().await;
        *is_running = false;

        info!("Delta Lake exporter '{}' shutdown completed", self.name);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Delta Lake exporter example");

    // Create pipeline configuration
    let config = PipelineConfig {
        name: "delta-lake-pipeline".to_string(),
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

    // Create and add receiver
    let mut receiver = SimpleMockReceiver::new("delta-lake-receiver".to_string());
    receiver.is_running = true; // Make receiver healthy
    let receiver = Arc::new(receiver);
    pipeline.add_receiver(receiver);

    // Create and add Delta Lake exporter
    let delta_exporter = DeltaLakeExporter::new("delta-lake-exporter".to_string());
    let delta_exporter = Arc::new(delta_exporter);
    pipeline.add_exporter(delta_exporter);

    // Start pipeline
    pipeline.start().await?;

    info!("Pipeline started successfully");

    // Let it run for a few seconds
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Stop pipeline
    pipeline.stop().await?;

    info!("Delta Lake exporter example completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_delta_lake_exporter_creation() {
        let exporter = DeltaLakeExporter::new("test-exporter".to_string());
        assert_eq!(exporter.name(), "test-exporter");
        assert_eq!(exporter.version(), "1.0.0");
    }

    #[tokio::test]
    async fn test_delta_lake_exporter_lifecycle() {
        let mut exporter = DeltaLakeExporter::new("test-exporter".to_string());

        // Initially running (as per implementation)
        assert!(exporter.health_check().await.unwrap());

        // Stop exporter
        exporter.stop().await;
        assert!(!exporter.health_check().await.unwrap());

        // Start exporter again
        exporter.start().await;
        assert!(exporter.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_delta_lake_exporter_export() {
        let mut exporter = DeltaLakeExporter::new("test-exporter".to_string());
        exporter.start().await;

        // Create a test batch
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

        let processed_batch = ProcessedBatch {
            original_batch_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            records: vec![processed_record],
            metadata: HashMap::new(),
            status: ProcessingStatus::Success,
            errors: Vec::new(),
        };

        // Export the batch
        let result = exporter.export(processed_batch).await.unwrap();
        assert_eq!(result.status, ExportStatus::Success);
        assert_eq!(result.records_exported, 1);
        assert_eq!(result.records_failed, 0);
    }

    #[tokio::test]
    async fn test_delta_lake_exporter_stats() {
        let mut exporter = DeltaLakeExporter::new("test-exporter".to_string());
        exporter.start().await;

        // Export a batch to update stats
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

        let processed_batch = ProcessedBatch {
            original_batch_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            records: vec![processed_record],
            metadata: HashMap::new(),
            status: ProcessingStatus::Success,
            errors: Vec::new(),
        };

        exporter.export(processed_batch).await.unwrap();

        // Check stats
        let stats = exporter.get_stats().await.unwrap();
        assert_eq!(stats.total_batches, 1);
        assert_eq!(stats.total_records, 1);
        assert!(stats.last_export_time.is_some());
    }
}
