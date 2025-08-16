//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating batch compression functionality
//!
//! This example shows how to use the batch processor with compression enabled
//! to reduce the size of telemetry batches.

use bridge_core::types::{
    MetricData, MetricType, MetricValue, TelemetryBatch, TelemetryData, TelemetryRecord,
    TelemetryType,
};
use bridge_core::BridgeResult;
use chrono::Utc;
use ingestion::processors::{batch_processor::BatchProcessorConfig, BatchProcessor};
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Batch Compression Example ===\n");

    // Create a batch processor with compression enabled
    let config = BatchProcessorConfig {
        name: "compression-example".to_string(),
        version: "1.0.0".to_string(),
        max_batch_size: 1000,
        max_batch_time_ms: 5000,
        flush_interval_ms: 1000,
        enable_compression: true,
        compression_level: 6, // zstd compression level (1-22)
        additional_config: HashMap::new(),
    };

    let processor = BatchProcessor::new(&config).await?;
    println!("Created batch processor with compression enabled");
    println!("Compression level: {}", config.compression_level);
    println!();

    // Create a large batch with many records to demonstrate compression
    let mut records = Vec::new();

    // Generate 100 sample metric records
    for i in 0..100 {
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: format!("example_metric_{}", i),
                description: Some(format!(
                    "Example metric number {} for compression testing",
                    i
                )),
                unit: Some("count".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(i as f64 * 1.5),
                labels: HashMap::from([
                    ("service".to_string(), "compression-example".to_string()),
                    ("instance".to_string(), format!("instance-{}", i % 5)),
                    ("version".to_string(), "1.0.0".to_string()),
                ]),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::from([
                (
                    "source".to_string(),
                    "batch-compression-example".to_string(),
                ),
                ("batch_id".to_string(), format!("batch-{}", i / 10)),
                ("record_index".to_string(), i.to_string()),
            ]),
            tags: HashMap::from([
                ("environment".to_string(), "development".to_string()),
                ("region".to_string(), "us-west-1".to_string()),
            ]),
            resource: None,
            service: None,
        };
        records.push(record);
    }

    let batch = TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        source: "compression-example".to_string(),
        size: records.len(),
        records,
        metadata: HashMap::from([
            ("example_type".to_string(), "compression_demo".to_string()),
            (
                "created_by".to_string(),
                "batch_compression_example".to_string(),
            ),
        ]),
    };

    println!("Created batch with {} records", batch.size);
    println!("Original batch ID: {}", batch.id);
    println!();

    // Test compression
    println!("Compressing batch...");
    let compressed_batch = processor.compress_batch(batch).await?;

    // Display compression results
    println!("Compression completed!");
    println!("Compressed batch ID: {}", compressed_batch.id);
    println!();

    // Show compression metadata
    if let Some(compression_type) = compressed_batch.metadata.get("compression") {
        println!("Compression type: {}", compression_type);
    }

    if let Some(compression_level) = compressed_batch.metadata.get("compression_level") {
        println!("Compression level: {}", compression_level);
    }

    if let Some(original_size) = compressed_batch.metadata.get("original_size") {
        println!("Original size: {} bytes", original_size);
    }

    if let Some(compressed_size) = compressed_batch.metadata.get("compressed_size") {
        println!("Compressed size: {} bytes", compressed_size);
    }

    if let Some(compression_ratio) = compressed_batch.metadata.get("compression_ratio") {
        println!("Compression ratio: {}", compression_ratio);

        // Calculate space savings
        if let (Ok(original), Ok(compressed)) = (
            compressed_batch.metadata["original_size"].parse::<usize>(),
            compressed_batch.metadata["compressed_size"].parse::<usize>(),
        ) {
            let savings = original - compressed;
            let savings_percent = (savings as f64 / original as f64) * 100.0;
            println!("Space saved: {} bytes ({:.1}%)", savings, savings_percent);
        }
    }

    println!();

    // Test with compression disabled
    println!("=== Testing with compression disabled ===");
    let config_disabled = BatchProcessorConfig {
        name: "no-compression-example".to_string(),
        version: "1.0.0".to_string(),
        max_batch_size: 1000,
        max_batch_time_ms: 5000,
        flush_interval_ms: 1000,
        enable_compression: false,
        compression_level: 6,
        additional_config: HashMap::new(),
    };

    let processor_disabled = BatchProcessor::new(&config_disabled).await?;
    let uncompressed_batch = processor_disabled
        .compress_batch(compressed_batch.clone())
        .await?;

    println!("Compression disabled - batch returned unchanged");
    println!("Batch ID: {}", uncompressed_batch.id);
    println!(
        "Has compression metadata: {}",
        uncompressed_batch.metadata.contains_key("compression")
    );

    println!("\n=== Example completed successfully! ===");

    Ok(())
}
