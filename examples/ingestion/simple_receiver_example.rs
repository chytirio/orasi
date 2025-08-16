//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating a simple receiver with bridge-core pipeline
//!
//! This example shows how to create a simple receiver and integrate it
//! with the bridge-core pipeline.

use async_trait::async_trait;
use bridge_core::{
    pipeline::PipelineConfig,
    traits::{ExporterStats, ReceiverStats},
    types::{
        ExportResult, MetricData, MetricValue, ProcessedBatch, TelemetryBatch, TelemetryData,
        TelemetryRecord, TelemetryType,
    },
    BridgeResult, LakehouseExporter, TelemetryIngestionPipeline, TelemetryReceiver,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

/// Simple mock receiver for demonstration
pub struct SimpleMockReceiver {
    name: String,
    is_running: bool,
    stats: ReceiverStats,
}

impl SimpleMockReceiver {
    pub fn new(name: String) -> Self {
        Self {
            name,
            is_running: false,
            stats: ReceiverStats {
                total_records: 0,
                records_per_minute: 0,
                total_bytes: 0,
                bytes_per_minute: 0,
                error_count: 0,
                last_receive_time: None,
            },
        }
    }

    pub fn start(&mut self) {
        self.is_running = true;
        info!("Simple mock receiver '{}' started", self.name);
    }

    pub fn stop(&mut self) {
        self.is_running = false;
        info!("Simple mock receiver '{}' stopped", self.name);
    }
}

#[async_trait]
impl TelemetryReceiver for SimpleMockReceiver {
    async fn receive(&self) -> BridgeResult<TelemetryBatch> {
        if !self.is_running {
            return Err(bridge_core::BridgeError::internal(
                "Receiver is not running",
            ));
        }

        // Create a sample batch
        let records = vec![TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: format!("{}_metric", self.name),
                description: Some(format!("Sample metric from {}", self.name)),
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
        }];

        let batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: self.name.clone(),
            size: records.len(),
            records,
            metadata: HashMap::new(),
        };

        info!(
            "Simple mock receiver '{}' received batch with {} records",
            self.name, batch.size
        );
        Ok(batch)
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        Ok(self.is_running)
    }

    async fn get_stats(&self) -> BridgeResult<ReceiverStats> {
        Ok(self.stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Simple mock receiver '{}' shutting down", self.name);
        Ok(())
    }
}

/// Simple mock exporter for demonstration
pub struct SimpleMockExporter {
    name: String,
    is_running: bool,
}

impl SimpleMockExporter {
    pub fn new(name: String) -> Self {
        Self {
            name,
            is_running: false,
        }
    }

    pub fn start(&mut self) {
        self.is_running = true;
        info!("Simple mock exporter '{}' started", self.name);
    }

    pub fn stop(&mut self) {
        self.is_running = false;
        info!("Simple mock exporter '{}' stopped", self.name);
    }
}

#[async_trait]
impl LakehouseExporter for SimpleMockExporter {
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        if !self.is_running {
            return Err(bridge_core::BridgeError::internal(
                "Exporter is not running",
            ));
        }

        info!(
            "Simple mock exporter '{}' exporting batch with {} records",
            self.name,
            batch.records.len()
        );

        // Simulate successful export
        Ok(ExportResult {
            timestamp: Utc::now(),
            status: bridge_core::types::ExportStatus::Success,
            records_exported: batch.records.len(),
            records_failed: 0,
            duration_ms: 10,
            metadata: HashMap::new(),
            errors: Vec::new(),
        })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        Ok(self.is_running)
    }

    async fn get_stats(&self) -> BridgeResult<ExporterStats> {
        Ok(ExporterStats {
            total_batches: 0,
            total_records: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            avg_export_time_ms: 0.0,
            error_count: 0,
            last_export_time: None,
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Simple mock exporter '{}' shutting down", self.name);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Simple Receiver Example");
    info!("===================================");

    // Create pipeline configuration
    let pipeline_config = PipelineConfig::default();
    let mut pipeline = TelemetryIngestionPipeline::new(pipeline_config);

    // Create simple mock receiver
    let mut receiver = SimpleMockReceiver::new("mock-http-receiver".to_string());

    info!("📡 Creating simple mock receiver: {}", receiver.name);

    // Start the receiver
    receiver.start();
    info!("✅ Simple mock receiver started successfully");

    // Create simple mock exporter
    let mut exporter = SimpleMockExporter::new("mock-lakehouse-exporter".to_string());

    info!("📤 Creating simple mock exporter: {}", exporter.name);

    // Start the exporter
    exporter.start();
    info!("✅ Simple mock exporter started successfully");

    // Add receiver and exporter to pipeline
    pipeline.add_receiver(Arc::new(receiver));
    pipeline.add_exporter(Arc::new(exporter));
    info!("✅ Receiver and exporter added to pipeline");

    // Start the pipeline
    pipeline.start().await?;
    info!("✅ Pipeline started successfully");

    // Simulate receiving data
    info!("⚡ Simulating telemetry data reception...");

    for i in 1..=5 {
        info!("   Processing batch {}...", i);

        // Create a sample batch for processing
        let sample_batch = create_sample_batch(i);

        // Process the batch through the pipeline
        match pipeline.process_batch(sample_batch).await {
            Ok(result) => {
                info!(
                    "   ✅ Batch {} processed successfully: {} records exported",
                    i, result.records_exported
                );
            }
            Err(e) => {
                error!("   ❌ Failed to process batch {}: {}", i, e);
            }
        }

        // Small delay between batches
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Get pipeline statistics
    let stats = pipeline.get_stats().await;
    info!("📊 Pipeline Statistics: {:?}", stats);

    // Get pipeline state
    let state = pipeline.get_state().await;
    info!("📈 Pipeline State: {:?}", state);

    // Stop the pipeline
    pipeline.stop().await?;
    info!("✅ Pipeline stopped successfully");

    info!("🎉 Simple Receiver Example completed successfully!");
    info!("=================================================");

    Ok(())
}

/// Create a sample telemetry batch for demonstration
fn create_sample_batch(batch_id: u32) -> TelemetryBatch {
    let records = vec![
        TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: format!("example_metric_{}", batch_id),
                description: Some(format!("Example metric from batch {}", batch_id)),
                unit: Some("count".to_string()),
                metric_type: bridge_core::types::MetricType::Gauge,
                value: MetricValue::Gauge(batch_id as f64),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        },
        TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: format!("example_counter_{}", batch_id),
                description: Some(format!("Example counter from batch {}", batch_id)),
                unit: Some("count".to_string()),
                metric_type: bridge_core::types::MetricType::Counter,
                value: MetricValue::Counter(batch_id as f64 * 10.0),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        },
    ];

    TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        source: "example".to_string(),
        size: records.len(),
        records,
        metadata: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_mock_receiver() {
        let mut receiver = SimpleMockReceiver::new("test-receiver".to_string());

        // Initially not running
        assert!(!receiver.is_running);

        // Start receiver
        receiver.start();
        assert!(receiver.is_running);

        // Health check should pass
        assert!(receiver.health_check().await.unwrap());

        // Receive data
        let batch = receiver.receive().await.unwrap();
        assert_eq!(batch.source, "test-receiver");
        assert_eq!(batch.size, 1);

        // Stop receiver
        receiver.stop();
        assert!(!receiver.is_running);
    }

    #[tokio::test]
    async fn test_simple_mock_exporter() {
        let mut exporter = SimpleMockExporter::new("test-exporter".to_string());

        // Initially not running
        assert!(!exporter.is_running);

        // Start exporter
        exporter.start();
        assert!(exporter.is_running);

        // Health check should pass
        assert!(exporter.health_check().await.unwrap());

        // Create a test batch
        let test_batch = ProcessedBatch {
            original_batch_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            status: bridge_core::types::ProcessingStatus::Success,
            records: vec![],
            metadata: HashMap::new(),
            errors: vec![],
        };

        // Export data
        let result = exporter.export(test_batch).await.unwrap();
        assert_eq!(result.records_exported, 0); // Empty batch has 0 records

        // Stop exporter
        exporter.stop();
        assert!(!exporter.is_running);
    }

    #[tokio::test]
    async fn test_sample_batch_creation() {
        let batch = create_sample_batch(1);

        assert_eq!(batch.source, "example");
        assert_eq!(batch.size, 2);
        assert_eq!(batch.records.len(), 2);

        // Check first record
        let record = &batch.records[0];
        assert_eq!(record.record_type, TelemetryType::Metric);

        if let TelemetryData::Metric(metric) = &record.data {
            assert_eq!(metric.name, "example_metric_1");
            assert_eq!(
                metric.description,
                Some("Example metric from batch 1".to_string())
            );
        } else {
            panic!("Expected metric data");
        }
    }
}
