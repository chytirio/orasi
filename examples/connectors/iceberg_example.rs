//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Iceberg connector example
//! 
//! This example demonstrates how to use the Apache Iceberg connector
//! to export telemetry data to Iceberg tables.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};
use bridge_core::pipeline::TelemetryIngestionPipeline;
use bridge_core::traits::{TelemetryReceiver, LakehouseExporter};
use bridge_core::types::{TelemetryBatch, MetricsBatch, MetricRecord};
use bridge_core::error::BridgeResult;

use iceberg::IcebergExporter;
use iceberg::config::IcebergConfig;

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
    async fn receive(&mut self) -> BridgeResult<Option<TelemetryBatch>> {
        if self.batch_count >= self.max_batches {
            return Ok(None);
        }

        self.batch_count += 1;
        info!("Mock receiver generating batch {}", self.batch_count);

        // Create sample metrics
        let metrics = MetricsBatch {
            records: vec![
                MetricRecord {
                    name: format!("cpu_usage_batch_{}", self.batch_count),
                    value: 75.0 + (self.batch_count as f64 * 0.5),
                    timestamp: chrono::Utc::now(),
                    labels: vec![
                        ("service".to_string(), "web-server".to_string()),
                        ("instance".to_string(), "web-1".to_string()),
                        ("batch".to_string(), self.batch_count.to_string()),
                    ],
                },
                MetricRecord {
                    name: format!("memory_usage_batch_{}", self.batch_count),
                    value: 60.0 + (self.batch_count as f64 * 0.3),
                    timestamp: chrono::Utc::now(),
                    labels: vec![
                        ("service".to_string(), "web-server".to_string()),
                        ("instance".to_string(), "web-1".to_string()),
                        ("batch".to_string(), self.batch_count.to_string()),
                    ],
                },
                MetricRecord {
                    name: format!("request_count_batch_{}", self.batch_count),
                    value: 1000.0 + (self.batch_count as f64 * 100.0),
                    timestamp: chrono::Utc::now(),
                    labels: vec![
                        ("service".to_string(), "web-server".to_string()),
                        ("instance".to_string(), "web-1".to_string()),
                        ("batch".to_string(), self.batch_count.to_string()),
                    ],
                },
                MetricRecord {
                    name: format!("error_rate_batch_{}", self.batch_count),
                    value: 2.5 + (self.batch_count as f64 * 0.1),
                    timestamp: chrono::Utc::now(),
                    labels: vec![
                        ("service".to_string(), "web-server".to_string()),
                        ("instance".to_string(), "web-1".to_string()),
                        ("batch".to_string(), self.batch_count.to_string()),
                    ],
                },
            ],
        };

        let batch = TelemetryBatch {
            metrics: Some(metrics),
            traces: None,
            logs: None,
        };

        Ok(Some(batch))
    }

    fn name(&self) -> &str {
        "mock-telemetry-receiver"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }
}

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("Starting Apache Iceberg connector example");

    // Create Iceberg configuration
    let iceberg_config = IcebergConfig {
        table_name: "telemetry_metrics".to_string(),
        warehouse_location: "s3://iceberg-warehouse".to_string(),
        catalog_type: "hive".to_string(),
        compression_codec: "snappy".to_string(),
        batch_size: 1000,
        flush_interval_ms: 5000,
        partition_columns: vec!["year".to_string(), "month".to_string(), "day".to_string(), "hour".to_string()],
        format_version: 2,
        snapshot_retention_days: 30,
        ..Default::default()
    };

    info!("Iceberg configuration: table={}, warehouse={}, catalog={}",
          iceberg_config.table_name, iceberg_config.warehouse_location, iceberg_config.catalog_type);

    // Create Iceberg exporter
    let mut iceberg_exporter = IcebergExporter::new(iceberg_config);
    
    // Initialize the exporter
    match iceberg_exporter.initialize().await {
        Ok(()) => info!("Iceberg exporter initialized successfully"),
        Err(e) => {
            error!("Failed to initialize Iceberg exporter: {}", e);
            return Err(bridge_core::error::BridgeError::exporter(format!("Failed to initialize Iceberg exporter: {}", e)));
        }
    }

    // Create mock receiver
    let mut receiver = MockTelemetryReceiver::new(5);

    // Create pipeline
    let mut pipeline = TelemetryIngestionPipeline::new(
        Box::new(receiver),
        Box::new(iceberg_exporter),
    );

    info!("Starting telemetry ingestion pipeline with Iceberg exporter");

    // Run the pipeline
    let pipeline_result = pipeline.run().await;

    match pipeline_result {
        Ok(stats) => {
            info!("Pipeline completed successfully");
            info!("Pipeline statistics:");
            info!("  Total batches processed: {}", stats.total_batches);
            info!("  Total records processed: {}", stats.total_records);
            info!("  Total processing time: {:?}", stats.total_processing_time);
            info!("  Average batch processing time: {:?}", stats.average_batch_processing_time);
        }
        Err(e) => {
            error!("Pipeline failed: {}", e);
            return Err(e);
        }
    }

    info!("Apache Iceberg connector example completed successfully");
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
