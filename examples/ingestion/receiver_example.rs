//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating the simple HTTP receiver with bridge-core pipeline
//!
//! This example shows how to create a simple HTTP receiver and integrate it
//! with the bridge-core pipeline.

use bridge_core::pipeline::PipelineConfig;
use bridge_core::{
    types::{
        MetricData, MetricType, MetricValue, TelemetryBatch, TelemetryData, TelemetryRecord,
        TelemetryType,
    },
    BridgeResult, TelemetryIngestionPipeline,
};
use ingestion::receivers::{SimpleHttpReceiver, SimpleHttpReceiverConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Simple HTTP Receiver Example");
    info!("==========================================");

    // Create pipeline configuration
    let pipeline_config = PipelineConfig::default();
    let mut pipeline = TelemetryIngestionPipeline::new(pipeline_config);

    // Create simple HTTP receiver configuration
    let receiver_config = SimpleHttpReceiverConfig::new("127.0.0.1".to_string(), 8080);
    let mut receiver = SimpleHttpReceiver::new(receiver_config);

    info!(
        "📡 Creating simple HTTP receiver on {}:{}",
        receiver.get_endpoint(),
        receiver.get_port()
    );

    // Start the receiver
    receiver.start().await?;
    info!("✅ Simple HTTP receiver started successfully");

    // Add receiver to pipeline
    pipeline.add_receiver(Arc::new(receiver));
    info!("✅ Receiver added to pipeline");

    // Start the pipeline
    pipeline.start().await?;
    info!("✅ Pipeline started successfully");

    // Simulate receiving data
    info!("⚡ Simulating telemetry data reception...");

    for i in 1..=5 {
        info!("   Processing batch {}...", i);

        // In a real scenario, the receiver would receive data from HTTP requests
        // For now, we'll just simulate by calling receive directly
        if let Ok(batch) = pipeline.process_batch(create_sample_batch(i)).await {
            info!(
                "   ✅ Batch {} processed successfully: {} records",
                i, batch.records_exported
            );
        } else {
            error!("   ❌ Failed to process batch {}", i);
        }

        // Small delay between batches
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Get pipeline statistics
    let stats = pipeline.get_stats().await;
    info!("📊 Pipeline Statistics: {:?}", stats);

    // Stop the pipeline
    pipeline.stop().await?;
    info!("✅ Pipeline stopped successfully");

    info!("🎉 Simple HTTP Receiver Example completed successfully!");
    info!("=====================================================");

    Ok(())
}

/// Create a sample telemetry batch for demonstration
fn create_sample_batch(batch_id: u32) -> TelemetryBatch {
    let records = vec![
        TelemetryRecord {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: format!("example_metric_{}", batch_id),
                description: Some(format!("Example metric from batch {}", batch_id)),
                unit: Some("count".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(batch_id as f64),
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        },
        TelemetryRecord {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: format!("example_counter_{}", batch_id),
                description: Some(format!("Example counter from batch {}", batch_id)),
                unit: Some("count".to_string()),
                metric_type: MetricType::Counter,
                value: MetricValue::Counter(batch_id as f64 * 10.0),
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        },
    ];

    TelemetryBatch {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
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
    async fn test_receiver_example_creation() {
        let receiver_config = SimpleHttpReceiverConfig::new("127.0.0.1".to_string(), 8080);
        let mut receiver = SimpleHttpReceiver::new(receiver_config);

        assert_eq!(receiver.get_endpoint(), "127.0.0.1");
        assert_eq!(receiver.get_port(), 8080);

        // Test lifecycle
        receiver.start().await.unwrap();
        assert!(receiver.is_running().await);

        receiver.stop().await.unwrap();
        assert!(!receiver.is_running().await);
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
