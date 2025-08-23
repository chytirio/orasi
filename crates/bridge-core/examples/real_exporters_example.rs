//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating real data lakehouse exporters
//!
//! This example shows how to use the real Parquet and Delta Lake exporters
//! to export telemetry data to data lakehouse formats.

use bridge_core::{
    exporters::delta::{DeltaLakeExporter, DeltaLakeExporterConfig, DeltaWriteMode},
    exporters::mock::{MockExporter, MockExporterConfig},
    exporters::parquet::{ParquetCompression, ParquetExporter, ParquetExporterConfig},
    health::HealthCheckConfig,
    metrics::MetricsConfig,
    pipeline::PipelineConfig,
    pipeline::TelemetryIngestionPipeline,
    processors::filter::{FilterProcessor, FilterProcessorConfig},
    processors::mock::{MockProcessor, MockProcessorConfig},
    receivers::mock::{MockReceiver, MockReceiverConfig},
    types::{
        ProcessedBatch, ProcessingStatus, TelemetryBatch, TelemetryData, TelemetryRecord,
        TelemetryType,
    },
    BridgeConfig, BridgeSystem, LakehouseExporter, TelemetryProcessor, TelemetryReceiver,
};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Starting real exporters example...");

    // Create temporary directories for data
    let temp_dir = TempDir::new()?;
    let parquet_dir = temp_dir.path().join("parquet");
    let delta_dir = temp_dir.path().join("delta");
    std::fs::create_dir_all(&parquet_dir)?;
    std::fs::create_dir_all(&delta_dir)?;

    println!("Using temporary directories:");
    println!("  Parquet: {}", parquet_dir.display());
    println!("  Delta: {}", delta_dir.display());

    // Create bridge system
    let bridge_config = BridgeConfig::default();

    let bridge_system = BridgeSystem::new(bridge_config).await?;
    println!("Created bridge system");

    // Create receivers
    let mock_receiver_config = MockReceiverConfig {
        records_per_batch: 10,
        error_probability: 0.0,
        auto_generate: true,
        generation_interval_ms: 100,
        simulate_errors: false,
    };
    let mock_receiver = Arc::new(MockReceiver::new(mock_receiver_config));
    println!("Created mock receiver");

    // Create processors
    let filter_config = FilterProcessorConfig {
        filters: vec![bridge_core::types::queries::filters::Filter {
            field: "service".to_string(),
            operator: bridge_core::types::queries::filters::FilterOperator::Equals,
            value: bridge_core::types::queries::filters::FilterValue::String(
                "web-service".to_string(),
            ),
        }],
        drop_unmatched: false,
        match_any: false,
        max_records_per_batch: Some(1000),
    };
    let filter_processor = Arc::new(FilterProcessor::new(filter_config));

    let mock_processor_config = MockProcessorConfig {
        simulate_delay: true,
        processing_delay_ms: 50,
        error_probability: 0.0,
        transform_data: true,
        add_metadata: true,
        simulate_errors: false,
    };
    let mock_processor = Arc::new(MockProcessor::new(mock_processor_config));
    println!("Created processors");

    // Create exporters
    let parquet_config = ParquetExporterConfig {
        output_directory: parquet_dir.to_string_lossy().to_string(),
        file_prefix: "telemetry".to_string(),
        max_records_per_file: 100,
        compression: ParquetCompression::Snappy,
        enable_partitioning: false,
        partition_columns: Vec::new(),
        create_directory: true,
    };
    let parquet_exporter = Arc::new(ParquetExporter::new(parquet_config));

    let delta_config = DeltaLakeExporterConfig {
        table_location: format!("file://{}", delta_dir.to_string_lossy()),
        table_name: "telemetry".to_string(),
        max_records_per_file: 100,
        partition_columns: vec!["date".to_string()],
        create_table_if_not_exists: true,
        write_mode: DeltaWriteMode::Append,
        storage_options: HashMap::new(),
    };
    let delta_exporter = Arc::new(DeltaLakeExporter::new(delta_config));

    let mock_exporter_config = MockExporterConfig {
        simulate_delay: true,
        export_delay_ms: 25,
        error_probability: 0.0,
        destination: "mock_destination".to_string(),
        backpressure_threshold: 1000,
        simulate_backpressure: false,
        simulate_errors: false,
    };
    let mock_exporter = Arc::new(MockExporter::new(mock_exporter_config));
    println!("Created exporters");

    // Create pipeline configuration
    let pipeline_config = PipelineConfig {
        name: "real_exporters_pipeline".to_string(),
        max_batch_size: 50,
        flush_interval_ms: 5000,
        buffer_size: 1000,
        enable_backpressure: true,
        backpressure_threshold: 80,
        enable_metrics: true,
        enable_health_checks: true,
        health_check_interval_ms: 1000,
    };

    // Create pipeline
    let mut pipeline = TelemetryIngestionPipeline::new(pipeline_config);

    // Add components to pipeline
    pipeline.add_receiver(mock_receiver.clone());
    pipeline.add_processor(filter_processor.clone());
    pipeline.add_processor(mock_processor.clone());
    pipeline.add_exporter(parquet_exporter.clone());
    pipeline.add_exporter(delta_exporter.clone());
    pipeline.add_exporter(mock_exporter.clone());

    let pipeline = Arc::new(RwLock::new(pipeline));
    println!("Created pipeline with components");

    // Add health checks
    bridge_system
        .health_checker
        .add_health_check(mock_receiver.clone())
        .await?;
    bridge_system
        .health_checker
        .add_health_check(filter_processor.clone())
        .await?;
    bridge_system
        .health_checker
        .add_health_check(mock_processor.clone())
        .await?;
    bridge_system
        .health_checker
        .add_health_check(parquet_exporter.clone())
        .await?;
    bridge_system
        .health_checker
        .add_health_check(delta_exporter.clone())
        .await?;
    bridge_system
        .health_checker
        .add_health_check(mock_exporter.clone())
        .await?;
    println!("Added health checks");

    // Start components
    mock_receiver.start().await?;
    filter_processor.start().await?;
    mock_processor.start().await?;
    parquet_exporter.start().await?;
    delta_exporter.start().await?;
    mock_exporter.start().await?;
    println!("Started all components");

    // Set and start pipeline
    bridge_system.set_pipeline(pipeline.clone()).await?;
    bridge_system.start_pipeline().await?;
    println!("Started pipeline");

    // Wait for data to be processed
    println!("Processing data for 10 seconds...");
    sleep(Duration::from_secs(10)).await;

    // Get status
    let status = bridge_system.get_status().await;
    println!("Bridge status: {:?}", status);

    // Get component statistics
    println!("\nComponent Statistics:");

    let receiver_stats = mock_receiver.get_stats().await?;
    println!(
        "Mock Receiver: {} records, {} errors",
        receiver_stats.total_records, receiver_stats.error_count
    );

    let filter_stats = filter_processor.get_stats().await?;
    println!(
        "Filter Processor: {} records processed",
        filter_stats.total_records
    );

    let processor_stats = mock_processor.get_stats().await?;
    println!(
        "Mock Processor: {} records processed",
        processor_stats.total_records
    );

    let parquet_stats = parquet_exporter.get_stats().await?;
    println!(
        "Parquet Exporter: {} batches, {} records, {} files",
        parquet_stats.total_batches, parquet_stats.total_records, parquet_stats.total_batches
    );

    let delta_stats = delta_exporter.get_stats().await?;
    println!(
        "Delta Lake Exporter: {} batches, {} records",
        delta_stats.total_batches, delta_stats.total_records
    );

    let mock_exporter_stats = mock_exporter.get_stats().await?;
    println!(
        "Mock Exporter: {} batches, {} records",
        mock_exporter_stats.total_batches, mock_exporter_stats.total_records
    );

    // Check generated files
    println!("\nGenerated Files:");

    let parquet_files: Vec<_> = std::fs::read_dir(&parquet_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map_or(false, |ext| ext == "parquet")
        })
        .collect();
    println!("Parquet files: {} files", parquet_files.len());
    for file in &parquet_files {
        let metadata = std::fs::metadata(&file.path())?;
        println!(
            "  {}: {} bytes",
            file.file_name().to_string_lossy(),
            metadata.len()
        );
    }

    let delta_files: Vec<_> = std::fs::read_dir(&delta_dir)?
        .filter_map(|entry| entry.ok())
        .collect();
    println!("Delta files: {} files", delta_files.len());
    for file in &delta_files {
        let metadata = std::fs::metadata(&file.path())?;
        println!(
            "  {}: {} bytes",
            file.file_name().to_string_lossy(),
            metadata.len()
        );
    }

    // Stop pipeline
    bridge_system.stop_pipeline().await?;
    println!("Stopped pipeline");

    // Stop individual components
    mock_receiver.stop().await?;
    filter_processor.stop().await?;
    mock_processor.stop().await?;
    parquet_exporter.stop().await?;
    delta_exporter.stop().await?;
    mock_exporter.stop().await?;
    println!("Stopped all components");

    println!("Real exporters example completed successfully!");
    println!("Data was exported to:");
    println!("  Parquet: {}", parquet_dir.display());
    println!("  Delta: {}", delta_dir.display());

    Ok(())
}
