//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Delta Lake Exporter Example
//!
//! This example demonstrates how to use the Delta Lake exporter to write
//! telemetry data to Delta Lake format.

use bridge_core::exporters::{delta::DeltaLakeExporterConfig, DeltaLakeExporter};
use bridge_core::{
    exporters::delta::DeltaWriteMode,
    types::{
        MetricData, MetricType, MetricValue, TelemetryBatch, TelemetryData, TelemetryRecord,
        TelemetryType,
    },
    types::{
        ProcessedBatch, ProcessedRecord, ProcessingStatus, SpanKind, SpanStatus, StatusCode,
        TraceData,
    },
    BridgeResult, LakehouseExporter,
};
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Delta Lake Exporter Example");

    // Create a temporary directory for Delta Lake tables
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let delta_base_path = temp_dir.path().to_str().unwrap();

    info!("Using temporary Delta Lake directory: {}", delta_base_path);

    // Create Delta Lake exporter configuration
    let config = DeltaLakeExporterConfig {
        table_location: delta_base_path.to_string(),
        table_name: "telemetry".to_string(),
        max_records_per_file: 1000,
        partition_columns: vec!["service_name".to_string(), "date".to_string()],
        create_table_if_not_exists: true,
        write_mode: DeltaWriteMode::Append,
        storage_options: HashMap::new(),
    };

    // Create Delta Lake exporter
    let exporter = DeltaLakeExporter::new(config);
    info!("✓ Delta Lake exporter created");

    // Start the exporter
    exporter.start().await?;
    info!("✓ Delta Lake exporter started");

    // Create sample telemetry data
    let sample_batches = create_sample_batches();
    info!("✓ Created {} sample batches", sample_batches.len());

    // Export batches
    for (i, batch) in sample_batches.into_iter().enumerate() {
        info!(
            "Exporting batch {} with {} records",
            i + 1,
            batch.records.len()
        );

        let result = LakehouseExporter::export(&exporter, batch).await?;
        info!(
            "✓ Batch {} exported successfully: {} records, {} bytes, {}ms",
            i + 1,
            result.records_exported,
            result
                .metadata
                .get("bytes_exported")
                .unwrap_or(&"unknown".to_string()),
            result.duration_ms
        );
    }

    // Get exporter statistics
    let stats = LakehouseExporter::get_stats(&exporter).await?;
    info!("Exporter stats: {:?}", stats);

    // Check exporter health
    let health = LakehouseExporter::health_check(&exporter).await?;
    info!(
        "✓ Exporter health check: {}",
        if health { "healthy" } else { "unhealthy" }
    );

    // Stop the exporter
    exporter.stop().await?;
    info!("✓ Delta Lake exporter stopped");

    info!("Delta Lake Exporter Example completed successfully");
    info!(
        "Check the output directory for generated Parquet files: {}",
        delta_base_path
    );

    Ok(())
}

/// Create sample telemetry batches
fn create_sample_batches() -> Vec<ProcessedBatch> {
    let mut batches = Vec::new();

    // Create multiple batches with different service names
    let services = vec![
        "user-service",
        "auth-service",
        "payment-service",
        "notification-service",
    ];

    for (batch_idx, service_name) in services.iter().enumerate() {
        let mut records = Vec::new();

        // Create multiple records per batch
        for record_idx in 0..10 {
            let trace_data = TraceData {
                trace_id: format!("trace-{}-{}", batch_idx, record_idx),
                span_id: format!("span-{}-{}", batch_idx, record_idx),
                parent_span_id: None,
                name: format!("{}-operation", service_name),
                kind: SpanKind::Internal,
                start_time: Utc::now(),
                end_time: Some(Utc::now()),
                duration_ns: Some(1000000), // 1ms
                status: SpanStatus {
                    code: StatusCode::Ok,
                    message: None,
                },
                attributes: HashMap::from([
                    ("service.name".to_string(), service_name.to_string()),
                    (
                        "operation.name".to_string(),
                        format!("{}-operation", service_name),
                    ),
                    ("duration_ms".to_string(), "1.0".to_string()),
                    ("status_code".to_string(), "200".to_string()),
                ]),
                events: Vec::new(),
                links: Vec::new(),
            };

            let mut metadata = HashMap::new();
            metadata.insert(
                "timestamp".to_string(),
                Utc::now().timestamp_nanos_opt().unwrap_or(0).to_string(),
            );
            metadata.insert("record_type".to_string(), "trace".to_string());
            metadata.insert("service_name".to_string(), service_name.to_string());
            metadata.insert(
                "operation_name".to_string(),
                format!("{}-operation", service_name),
            );
            metadata.insert("duration_ms".to_string(), "1.0".to_string());
            metadata.insert("status_code".to_string(), "200".to_string());

            let record = ProcessedRecord {
                original_id: Uuid::new_v4(),
                status: ProcessingStatus::Success,
                transformed_data: Some(TelemetryData::Trace(trace_data)),
                metadata,
                errors: Vec::new(),
            };

            records.push(record);
        }

        let batch = ProcessedBatch {
            original_batch_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            status: ProcessingStatus::Success,
            records,
            metadata: HashMap::new(),
            errors: Vec::new(),
        };

        batches.push(batch);
    }

    batches
}
