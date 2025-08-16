//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming Exporter Example
//!
//! This example demonstrates the streaming exporter functionality with actual data streaming
//! to a destination URL.

use bridge_core::BridgeResult;
use ingestion::{
    exporters::streaming_exporter::{StreamingExporter, StreamingExporterConfig},
    types::{ProcessedBatch, ProcessedRecord, ProcessingStatus},
};
use std::collections::HashMap;
use tracing::{error, info};
use uuid::Uuid;
use chrono::Utc;

fn create_test_batch() -> ProcessedBatch {
    ProcessedBatch {
        original_batch_id: Uuid::new_v4(),
        timestamp: Utc::now(),
        status: ProcessingStatus::Success,
        records: vec![
            ProcessedRecord {
                original_id: Uuid::new_v4(),
                status: ProcessingStatus::Success,
                transformed_data: None,
                metadata: HashMap::from([
                    ("test_key".to_string(), "test_value".to_string()),
                    ("source".to_string(), "streaming_example".to_string()),
                ]),
                errors: vec![],
            },
        ],
        metadata: HashMap::from([
            ("source".to_string(), "streaming_example".to_string()),
            ("version".to_string(), "1.0.0".to_string()),
        ]),
        errors: vec![],
    }
}

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting streaming exporter example");

    // Create streaming exporter configuration
    // Note: This will attempt to stream to a local endpoint
    // In a real scenario, you would use an actual destination URL
    let config = StreamingExporterConfig::new(
        "http://localhost:8080/stream".to_string(),
        10, // Buffer size
    );

    // Create streaming exporter
    let exporter = StreamingExporter::new(&config).await?;
    info!("Created streaming exporter with destination: {}", config.destination_url);

    // Create test batch
    let batch = create_test_batch();
    info!("Created test batch with {} records", batch.records.len());

    // Export the batch
    info!("Exporting batch...");
    let result = exporter.export(batch).await?;
    
    info!("Export completed with status: {:?}", result.status);
    info!("Records exported: {}", result.records_exported);
    info!("Records failed: {}", result.records_failed);
    info!("Export duration: {}ms", result.duration_ms);

    if !result.errors.is_empty() {
        for error in &result.errors {
            error!("Export error: {} - {}", error.code, error.message);
        }
    }

    // Get exporter statistics
    let stats = exporter.get_stats().await?;
    info!("Exporter statistics:");
    info!("  Total batches: {}", stats.total_batches);
    info!("  Total records: {}", stats.total_records);
    info!("  Batches per minute: {}", stats.batches_per_minute);
    info!("  Records per minute: {}", stats.records_per_minute);
    info!("  Average export time: {:.2}ms", stats.avg_export_time_ms);
    info!("  Error count: {}", stats.error_count);

    // Check exporter health
    let is_healthy = exporter.health_check().await?;
    info!("Exporter health check: {}", if is_healthy { "PASSED" } else { "FAILED" });

    // Force flush any remaining data
    exporter.force_flush().await?;
    info!("Forced flush completed");

    // Shutdown the exporter
    exporter.shutdown().await?;
    info!("Streaming exporter shutdown completed");

    info!("Streaming exporter example completed successfully");
    Ok(())
}
