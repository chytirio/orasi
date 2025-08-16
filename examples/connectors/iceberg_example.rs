//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Iceberg connector example
//!
//! This example demonstrates how to use the Apache Iceberg connector
//! to export telemetry data to Iceberg tables.

use bridge_core::error::BridgeResult;
use bridge_core::pipeline::{PipelineConfig, TelemetryIngestionPipeline};
use bridge_core::traits::{LakehouseExporter, TelemetryReceiver};
use bridge_core::types::{
    MetricData, MetricType, MetricValue, TelemetryBatch, TelemetryData, TelemetryRecord,
    TelemetryType,
};
use chrono::Utc;
use std::collections::HashMap;
use tracing::{error, info};
use uuid::Uuid;

/// Mock telemetry receiver for testing
struct MockTelemetryReceiver {
    batch_count: usize,
    max_batches: usize,
}

impl MockTelemetryReceiver {
    fn new(max_batches: usize) -> Self {
        Self {
            batch_count: 0,
            max_batches,
        }
    }
}

#[async_trait::async_trait]
impl TelemetryReceiver for MockTelemetryReceiver {
    async fn receive(&self) -> BridgeResult<TelemetryBatch> {
        if self.batch_count >= self.max_batches {
            return Err(bridge_core::BridgeError::ingestion(
                "No more batches to receive",
            ));
        }

        let batch_count = self.batch_count + 1;
        info!("Mock receiver generating batch {}", batch_count);

        // Create sample metrics
        let metrics = vec![
            MetricData {
                name: format!("cpu_usage_batch_{}", batch_count),
                description: Some("CPU usage percentage".to_string()),
                unit: Some("%".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(75.0 + (batch_count as f64 * 0.5)),
                labels: HashMap::from([
                    ("service".to_string(), "web-server".to_string()),
                    ("instance".to_string(), "web-1".to_string()),
                    ("batch".to_string(), batch_count.to_string()),
                ]),
                timestamp: Utc::now(),
            },
            MetricData {
                name: format!("memory_usage_batch_{}", batch_count),
                description: Some("Memory usage percentage".to_string()),
                unit: Some("%".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(60.0 + (batch_count as f64 * 0.3)),
                labels: HashMap::from([
                    ("service".to_string(), "web-server".to_string()),
                    ("instance".to_string(), "web-1".to_string()),
                    ("batch".to_string(), batch_count.to_string()),
                ]),
                timestamp: Utc::now(),
            },
            MetricData {
                name: format!("request_count_batch_{}", batch_count),
                description: Some("Request count".to_string()),
                unit: Some("requests".to_string()),
                metric_type: MetricType::Counter,
                value: MetricValue::Counter(1000.0 + (batch_count as f64 * 100.0)),
                labels: HashMap::from([
                    ("service".to_string(), "web-server".to_string()),
                    ("instance".to_string(), "web-1".to_string()),
                    ("batch".to_string(), batch_count.to_string()),
                ]),
                timestamp: Utc::now(),
            },
            MetricData {
                name: format!("error_rate_batch_{}", batch_count),
                description: Some("Error rate percentage".to_string()),
                unit: Some("%".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(2.5 + (batch_count as f64 * 0.1)),
                labels: HashMap::from([
                    ("service".to_string(), "web-server".to_string()),
                    ("instance".to_string(), "web-1".to_string()),
                    ("batch".to_string(), batch_count.to_string()),
                ]),
                timestamp: Utc::now(),
            },
        ];

        let records: Vec<TelemetryRecord> = metrics
            .into_iter()
            .map(|metric_data| TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(metric_data),
                attributes: HashMap::new(),
                tags: HashMap::new(),
                resource: None,
                service: None,
            })
            .collect();

        let batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "mock-receiver".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::from([
                ("batch_number".to_string(), batch_count.to_string()),
                ("source".to_string(), "mock-receiver".to_string()),
            ]),
        };

        Ok(batch)
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        Ok(true)
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ReceiverStats> {
        Ok(bridge_core::traits::ReceiverStats {
            total_records: self.batch_count as u64 * 4, // 4 metrics per batch
            records_per_minute: 0,
            total_bytes: 0,
            bytes_per_minute: 0,
            error_count: 0,
            last_receive_time: None,
            protocol_stats: None,
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Mock telemetry receiver shutting down");
        Ok(())
    }
}

/// Mock Iceberg exporter for testing
struct MockIcebergExporter {
    export_count: std::sync::Arc<tokio::sync::RwLock<usize>>,
}

impl MockIcebergExporter {
    fn new() -> Self {
        Self {
            export_count: std::sync::Arc::new(tokio::sync::RwLock::new(0)),
        }
    }
}

#[async_trait::async_trait]
impl LakehouseExporter for MockIcebergExporter {
    async fn export(
        &self,
        batch: bridge_core::types::ProcessedBatch,
    ) -> BridgeResult<bridge_core::types::ExportResult> {
        let mut count = self.export_count.write().await;
        *count += 1;

        info!(
            "Mock Iceberg exporter processing batch {} with {} records",
            batch.original_batch_id,
            batch.records.len()
        );

        // Simulate export processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(bridge_core::types::ExportResult {
            timestamp: Utc::now(),
            status: bridge_core::types::ExportStatus::Success,
            records_exported: batch.records.len(),
            records_failed: 0,
            duration_ms: 100,
            metadata: HashMap::from([
                ("exporter".to_string(), "mock-iceberg".to_string()),
                ("batch_count".to_string(), count.to_string()),
            ]),
            errors: vec![],
        })
    }

    fn name(&self) -> &str {
        "mock-iceberg-exporter"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        Ok(true)
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ExporterStats> {
        let count = *self.export_count.read().await;
        Ok(bridge_core::traits::ExporterStats {
            total_batches: count as u64,
            total_records: count as u64 * 4, // Assuming 4 records per batch
            batches_per_minute: 0,
            records_per_minute: 0,
            avg_export_time_ms: 100.0,
            error_count: 0,
            last_export_time: None,
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Mock Iceberg exporter shutting down");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Iceberg connector example");

    // Create pipeline configuration
    let pipeline_config = PipelineConfig {
        name: "iceberg-example-pipeline".to_string(),
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
    let mut pipeline = TelemetryIngestionPipeline::new(pipeline_config);

    // Create and add receiver
    let receiver = std::sync::Arc::new(MockTelemetryReceiver::new(5));
    pipeline.add_receiver(receiver);

    // Create and add exporter
    let exporter = std::sync::Arc::new(MockIcebergExporter::new());
    pipeline.add_exporter(exporter);

    // Start the pipeline
    pipeline.start().await?;
    info!("Pipeline started successfully");

    // Let the pipeline run for a bit
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // Stop the pipeline
    pipeline.stop().await?;
    info!("Pipeline stopped successfully");

    info!("Iceberg connector example completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_iceberg_example_components() {
        // Test Iceberg configuration
        let config = IcebergConfig {
            table_name: "test_table".to_string(),
            warehouse_location: "s3://test-warehouse".to_string(),
            catalog_type: "hive".to_string(),
            ..Default::default()
        };

        assert_eq!(config.table_name, "test_table");
        assert_eq!(config.warehouse_location, "s3://test-warehouse");
        assert_eq!(config.catalog_type, "hive");

        // Test mock receiver
        let mut receiver = MockTelemetryReceiver::new(2);
        let batch1 = receiver.receive().await.unwrap();
        assert!(batch1.is_some());

        let batch2 = receiver.receive().await.unwrap();
        assert!(batch2.is_some());

        let batch3 = receiver.receive().await.unwrap();
        assert!(batch3.is_none()); // Should return None after max_batches
    }
}
