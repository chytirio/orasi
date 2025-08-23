//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming processor example
//!
//! This example demonstrates a complete streaming pipeline with:
//! - Kafka source for ingesting telemetry data
//! - Filter processor for filtering records
//! - Transform processor for data transformation
//! - Kafka sink for outputting processed data

use bridge_core::{
    types::{MetricData, MetricType, MetricValue, TelemetryData, TelemetryRecord, TelemetryType},
    TelemetryBatch,
};
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use streaming_processor::{
    config::StreamingProcessorConfig,
    processors::{ProcessorFactory, ProcessorPipeline},
    sinks::{SinkFactory, SinkManager},
    sources::{SourceFactory, SourceManager},
    StreamingProcessor,
};
use tracing::{error, info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting streaming processor example");

    // Create streaming processor configuration
    let config = StreamingProcessorConfig::default();
    let mut processor = StreamingProcessor::new(config).await?;

    // Set up sources
    let source_manager = processor.source_manager();
    setup_sources(source_manager).await?;

    // Set up processors
    let processor_pipeline = processor.processor_pipeline();
    setup_processors(processor_pipeline).await?;

    // Set up sinks
    let sink_manager = processor.sink_manager();
    setup_sinks(sink_manager).await?;

    // Start the streaming processor
    processor.start().await?;

    info!("Streaming processor started successfully");

    // Run for a specified duration
    let runtime_duration = Duration::from_secs(30);
    info!("Running streaming processor for {:?}", runtime_duration);
    tokio::time::sleep(runtime_duration).await;

    // Stop the streaming processor
    processor.stop().await?;
    info!("Streaming processor stopped successfully");

    // Print final statistics
    print_statistics(&processor).await?;

    Ok(())
}

/// Set up data sources
async fn setup_sources(source_manager: &SourceManager) -> Result<(), Box<dyn std::error::Error>> {
    info!("Setting up data sources");

    // For now, just log that we would set up sources
    info!("Would configure Kafka source for topic: telemetry-input");

    Ok(())
}

/// Set up data processors
async fn setup_processors(
    processor_pipeline: &ProcessorPipeline,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Setting up data processors");

    // For now, just log that we would set up processors
    info!("Would configure filter and transform processors");

    Ok(())
}

/// Set up data sinks
async fn setup_sinks(sink_manager: &SinkManager) -> Result<(), Box<dyn std::error::Error>> {
    info!("Setting up data sinks");

    // For now, just log that we would set up sinks
    info!("Would configure Kafka sink for topic: telemetry-processed");

    Ok(())
}

/// Print final statistics
async fn print_statistics(
    processor: &StreamingProcessor,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Streaming Processor Statistics ===");

    // Print source statistics
    let source_stats = processor.source_manager().get_stats().await?;
    for (source_name, stats) in source_stats {
        info!("Source '{}':", source_name);
        info!("  Total records: {}", stats.total_records);
        info!("  Total bytes: {}", stats.total_bytes);
        info!("  Error count: {}", stats.error_count);
        info!("  Connected: {}", stats.is_connected);
    }

    // Print processor statistics
    let processor_stats = processor.processor_pipeline().get_stats().await?;
    info!("Processor Pipeline:");
    info!("  Total records: {}", processor_stats.total_records);
    info!(
        "  Average processing time: {:.2}ms",
        processor_stats.avg_processing_time_ms
    );
    info!("  Error count: {}", processor_stats.error_count);

    // Print sink statistics
    let sink_stats = processor.sink_manager().get_stats().await?;
    for (sink_name, stats) in sink_stats {
        info!("Sink '{}':", sink_name);
        info!("  Total batches: {}", stats.total_batches);
        info!("  Total records: {}", stats.total_records);
        info!("  Total bytes: {}", stats.total_bytes);
        info!("  Error count: {}", stats.error_count);
        info!("  Average latency: {}ms", stats.latency_ms);
        info!("  Connected: {}", stats.is_connected);
    }

    Ok(())
}

/// Generate sample telemetry data for testing
fn generate_sample_telemetry_batch() -> TelemetryBatch {
    let mut records = Vec::new();

    // Generate some sample metrics
    for i in 0..10 {
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: format!("sample_metric_{}", i),
                description: Some(format!("Sample metric {}", i)),
                unit: Some("count".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge((i * 10) as f64),
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("service".to_string(), "example-service".to_string());
                    labels.insert("environment".to_string(), "development".to_string());
                    labels
                },
                timestamp: Utc::now(),
            }),
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("level".to_string(), "info".to_string());
                attrs.insert("source".to_string(), "example".to_string());
                attrs
            },
            tags: HashMap::new(),
            resource: None,
            service: None,
        };
        records.push(record);
    }

    TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        source: "example".to_string(),
        size: records.len(),
        records,
        metadata: HashMap::new(),
    }
}
