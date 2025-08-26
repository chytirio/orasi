//! Delta Lake exporter demo functionality

use crate::error::BridgeResult;
use crate::types::{ProcessedBatch, ProcessedRecord, ProcessingStatus, TelemetryData};
use crate::traits::exporter::LakehouseExporter;
use super::config::DeltaLakeExporterConfig;
use super::exporter::DeltaLakeExporter;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

/// Run a demo of the Delta Lake exporter
pub async fn run_delta_lake_exporter_demo() -> BridgeResult<()> {
    info!("Starting Delta Lake Exporter Demo");

    // Create a temporary directory for the demo
    let delta_base_path = "./demo_delta_output";
    let table_location = format!("file://{}", delta_base_path);

    // Create exporter configuration
    let config = DeltaLakeExporterConfig {
        table_location,
        table_name: "telemetry_demo".to_string(),
        max_records_per_file: 1000,
        partition_columns: vec!["date".to_string()],
        create_table_if_not_exists: true,
        write_mode: super::config::DeltaWriteMode::Append,
        storage_options: HashMap::new(),
    };

    // Create the exporter
    let exporter = DeltaLakeExporter::new(config);

    // Start the exporter
    exporter.start().await?;
    info!("✓ Delta Lake exporter started");

    // Create sample batches
    let batches = create_sample_batches();
    info!("✓ Created {} sample batches", batches.len());

    // Export each batch
    for (i, batch) in batches.iter().enumerate() {
        info!("Exporting batch {}/{}", i + 1, batches.len());
        
        match exporter.export(batch.clone()).await {
            Ok(result) => {
                info!(
                    "✓ Batch {} exported successfully: {} records in {}ms",
                    i + 1,
                    result.records_exported,
                    result.duration_ms
                );
            }
            Err(e) => {
                info!("✗ Failed to export batch {}: {}", i + 1, e);
            }
        }
    }

    // Get statistics
    let stats = exporter.get_stats().await;
    info!("✓ Final statistics:");
    info!("  - Total batches: {}", stats.total_batches);
    info!("  - Total records: {}", stats.total_records);
    info!("  - Total bytes: {}", stats.total_bytes);
    info!("  - Files written: {}", stats.files_written);
    info!("  - Error count: {}", stats.error_count);

    // Stop the exporter
    exporter.stop().await?;
    info!("✓ Delta Lake exporter stopped");

    info!("Delta Lake Exporter Demo completed successfully");
    info!(
        "Check the output directory for generated Parquet files: {}",
        delta_base_path
    );

    Ok(())
}

/// Create sample telemetry batches for testing
pub fn create_sample_batches() -> Vec<ProcessedBatch> {
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
        for record_idx in 0..5 {
            let trace_data = crate::types::traces::TraceData {
                trace_id: format!("trace-{}-{}", batch_idx, record_idx),
                span_id: format!("span-{}-{}", batch_idx, record_idx),
                parent_span_id: None,
                name: format!("{}-operation", service_name),
                kind: crate::types::traces::SpanKind::Internal,
                start_time: chrono::Utc::now(),
                end_time: Some(chrono::Utc::now()),
                duration_ns: Some(1000000), // 1ms
                status: crate::types::traces::SpanStatus {
                    code: crate::types::traces::StatusCode::Ok,
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
                chrono::Utc::now()
                    .timestamp_nanos_opt()
                    .unwrap_or(0)
                    .to_string(),
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
            timestamp: chrono::Utc::now(),
            status: ProcessingStatus::Success,
            records,
            metadata: HashMap::new(),
            errors: Vec::new(),
        };

        batches.push(batch);
    }

    batches
}
