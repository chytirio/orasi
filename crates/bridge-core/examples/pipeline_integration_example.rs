//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Pipeline Integration Example
//!
//! This example demonstrates the complete pipeline integration with concrete
//! receivers, processors, and exporters working together in the bridge system.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{info, warn};

use bridge_core::{
    exporters::{mock::MockExporterConfig, MockExporter},
    pipeline::config::PipelineConfig,
    processors::{filter::FilterProcessorConfig, FilterProcessor, MockProcessor},
    receivers::{mock::MockReceiverConfig, MockReceiver},
    types::queries::filters::{Filter, FilterOperator, FilterValue},
    BridgeSystem, LakehouseExporter, TelemetryIngestionPipeline, TelemetryProcessor,
    TelemetryReceiver,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting pipeline integration example");

    // Initialize the bridge system
    let bridge_system = BridgeSystem::new(bridge_core::BridgeConfig::default()).await?;
    bridge_system.init().await?;
    bridge_system.start().await?;

    info!("Bridge system initialized and started");

    // Create pipeline configuration
    let pipeline_config = PipelineConfig {
        name: "example_pipeline".to_string(),
        max_batch_size: 1000,
        flush_interval_ms: 1000, // 1 second
        buffer_size: 1000,
        enable_backpressure: true,
        backpressure_threshold: 80, // 80%
        enable_metrics: true,
        enable_health_checks: true,
        health_check_interval_ms: 5000, // 5 seconds
    };

    // Create pipeline
    let mut pipeline = TelemetryIngestionPipeline::new(pipeline_config);

    // Create and configure receivers
    let mock_receiver_config = MockReceiverConfig {
        auto_generate: true,
        generation_interval_ms: 2000, // 2 seconds
        records_per_batch: 5,
        simulate_errors: false,
        error_probability: 0.0,
    };

    let mock_receiver = Arc::new(MockReceiver::new(mock_receiver_config));
    mock_receiver.start().await?;
    pipeline.add_receiver(mock_receiver.clone());

    info!("Added mock receiver to pipeline");

    // Create and configure processors
    let filter_config = FilterProcessorConfig {
        filters: vec![
            Filter {
                field: "service".to_string(),
                operator: FilterOperator::Equals,
                value: FilterValue::String("test_service".to_string()),
            },
            Filter {
                field: "environment".to_string(),
                operator: FilterOperator::Equals,
                value: FilterValue::String("production".to_string()),
            },
        ],
        match_any: true, // Match any filter (OR logic)
        drop_unmatched: true,
        max_records_per_batch: None,
    };

    let filter_processor = Arc::new(FilterProcessor::new(filter_config));
    filter_processor.start().await?;
    pipeline.add_processor(filter_processor.clone());

    let mock_processor_config = bridge_core::processors::mock::MockProcessorConfig {
        simulate_delay: true,
        processing_delay_ms: 100,
        simulate_errors: false,
        error_probability: 0.0,
        transform_data: true,
        add_metadata: true,
    };

    let mock_processor = Arc::new(MockProcessor::new(mock_processor_config));
    mock_processor.start().await?;
    pipeline.add_processor(mock_processor.clone());

    info!("Added filter and mock processors to pipeline");

    // Create and configure exporters
    let mock_exporter_config = MockExporterConfig {
        simulate_delay: true,
        export_delay_ms: 200,
        simulate_errors: false,
        error_probability: 0.0,
        simulate_backpressure: false,
        backpressure_threshold: 1000,
        destination: "mock://localhost/test".to_string(),
    };

    let mock_exporter = Arc::new(MockExporter::new(mock_exporter_config));
    mock_exporter.start().await?;
    pipeline.add_exporter(mock_exporter.clone());

    info!("Added mock exporter to pipeline");

    // Set the pipeline in the bridge system
    let pipeline_arc = Arc::new(RwLock::new(pipeline));
    bridge_system.set_pipeline(pipeline_arc.clone()).await?;
    info!("Pipeline set in bridge system");

    // Add a delay to ensure components are ready
    sleep(Duration::from_millis(2000)).await;

    // Check that components are healthy before starting pipeline
    let receiver_health = mock_receiver.health_check().await?;
    let filter_health = filter_processor.health_check().await?;
    let processor_health = mock_processor.health_check().await?;
    let exporter_health = mock_exporter.health_check().await?;

    info!(
        "Component health checks - Receiver: {}, Filter: {}, Processor: {}, Exporter: {}",
        receiver_health, filter_health, processor_health, exporter_health
    );

    if !receiver_health || !filter_health || !processor_health || !exporter_health {
        return Err("One or more components are not healthy".into());
    }

    // Test receiving data from mock receiver before starting pipeline
    match mock_receiver.receive().await {
        Ok(batch) => {
            info!(
                "Successfully received test batch with {} records",
                batch.records.len()
            );
        }
        Err(e) => {
            warn!("Failed to receive test batch: {}", e);
            return Err("Mock receiver is not working properly".into());
        }
    }

    // Start the pipeline through the bridge system
    bridge_system.start_pipeline().await?;
    info!("Pipeline started successfully through bridge system");

    // Add components to health monitoring
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
        .add_health_check(mock_exporter.clone())
        .await?;

    info!("Added all components to health monitoring");

    // Run the pipeline for a specified duration
    let run_duration = Duration::from_secs(30);
    info!("Running pipeline for {:?}...", run_duration);

    let start_time = std::time::Instant::now();
    let mut last_stats_time = start_time;

    while start_time.elapsed() < run_duration {
        // Check pipeline health
        let pipeline_health = if let Some(pipeline_arc) = bridge_system.get_pipeline().await {
            let pipeline = pipeline_arc.read().await;
            pipeline.health_check().await.unwrap_or(false)
        } else {
            false
        };

        if !pipeline_health {
            warn!("Pipeline health check failed");
        }

        // Print statistics every 5 seconds
        if last_stats_time.elapsed() >= Duration::from_secs(5) {
            let pipeline_stats = if let Some(pipeline_arc) = bridge_system.get_pipeline().await {
                let pipeline = pipeline_arc.read().await;
                pipeline.get_stats().await
            } else {
                bridge_core::pipeline::state::PipelineStats::default()
            };
            let bridge_status = bridge_system.get_status().await;

            info!("=== Pipeline Statistics ===");
            info!("Pipeline Health: {}", pipeline_health);
            info!("Total Batches: {}", pipeline_stats.total_batches);
            info!("Total Records: {}", pipeline_stats.total_records);
            info!(
                "Average Processing Time: {:.2}ms",
                pipeline_stats.avg_processing_time_ms
            );
            info!("Last Process Time: {:?}", pipeline_stats.last_process_time);

            info!("=== Bridge System Status ===");
            info!("Bridge Status: {}", bridge_status.status);
            info!("Uptime: {}s", bridge_status.uptime_seconds);
            info!("Active Components: {}", bridge_status.components.len());

            // Print component-specific stats
            let receiver_stats = mock_receiver.get_stats().await?;
            let processor_stats = mock_processor.get_stats().await?;
            let exporter_stats = mock_exporter.get_stats().await?;

            info!("=== Component Statistics ===");
            info!(
                "Receiver - Records: {}, Errors: {}",
                receiver_stats.total_records, receiver_stats.error_count
            );
            info!(
                "Processor - Records: {}, Errors: {}",
                processor_stats.total_records, processor_stats.error_count
            );
            info!(
                "Exporter - Records: {}, Errors: {}",
                exporter_stats.total_records, exporter_stats.error_count
            );

            last_stats_time = std::time::Instant::now();
        }

        sleep(Duration::from_millis(100)).await;
    }

    // Stop individual components first
    info!("Stopping individual components...");
    mock_receiver.stop().await?;
    filter_processor.stop().await?;
    mock_processor.stop().await?;
    mock_exporter.stop().await?;
    info!("All components stopped");

    // Stop the pipeline through the bridge system
    info!("Stopping pipeline...");
    bridge_system.stop_pipeline().await?;
    info!("Pipeline stopped");

    // Stop the bridge system
    bridge_system.stop().await?;
    info!("Bridge system stopped");

    // Print final statistics
    let final_pipeline_stats = if let Some(pipeline_arc) = bridge_system.get_pipeline().await {
        let pipeline = pipeline_arc.read().await;
        pipeline.get_stats().await
    } else {
        bridge_core::pipeline::state::PipelineStats::default()
    };
    let final_bridge_status = bridge_system.get_status().await;

    info!("=== Final Statistics ===");
    info!(
        "Total Batches Processed: {}",
        final_pipeline_stats.total_batches
    );
    info!(
        "Total Records Processed: {}",
        final_pipeline_stats.total_records
    );
    info!(
        "Total Processing Time: {}ms",
        final_pipeline_stats.total_processing_time_ms
    );
    info!(
        "Average Processing Time: {:.2}ms",
        final_pipeline_stats.avg_processing_time_ms
    );
    info!("Bridge Uptime: {}s", final_bridge_status.uptime_seconds);

    info!("Pipeline integration example completed successfully");
    Ok(())
}
