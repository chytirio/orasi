//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating advanced telemetry processors
//!
//! This example shows how to use the advanced Transform and Aggregate processors
//! to process and analyze telemetry data in sophisticated ways.

use bridge_core::{
    exporters::mock::{MockExporter, MockExporterConfig},
    pipeline::TelemetryIngestionPipeline,
    processors::aggregate::{
        AggregateProcessor, AggregateProcessorConfig, AggregationOperation, TimeWindow,
    },
    processors::transform::{
        DataType, TransformOperation, TransformProcessor, TransformProcessorConfig,
    },
    receivers::mock::{MockReceiver, MockReceiverConfig},
    BridgeConfig, BridgeSystem, LakehouseExporter, TelemetryProcessor, TelemetryReceiver,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Starting advanced processors example...");

    // Create bridge system with default configuration
    let bridge_config = BridgeConfig::default();
    let bridge_system = BridgeSystem::new(bridge_config).await?;
    println!("Created bridge system");

    // Create mock receiver that generates varied telemetry data
    let mock_receiver_config = MockReceiverConfig {
        records_per_batch: 5,
        error_probability: 0.0,
        auto_generate: true,
        generation_interval_ms: 200,
        simulate_errors: false,
    };
    let mock_receiver = Arc::new(MockReceiver::new(mock_receiver_config));
    println!("Created mock receiver");

    // Create advanced transform processor with various transformations
    let transform_config = TransformProcessorConfig {
        transformations: vec![
            // Add environment attribute to all records
            TransformOperation::AddAttribute {
                key: "environment".to_string(),
                value: "production".to_string(),
            },
            // Map service.name to service for compatibility
            TransformOperation::FieldMapping {
                from: "service.name".to_string(),
                to: "service".to_string(),
            },
            // Add processing timestamp
            TransformOperation::AddAttribute {
                key: "processed_at".to_string(),
                value: chrono::Utc::now().to_rfc3339(),
            },
            // Extract region from JSON metadata if available
            TransformOperation::JsonExtract {
                source_field: "metadata".to_string(),
                json_path: "region".to_string(),
                target_field: "region".to_string(),
            },
            // Add high_priority flag based on condition
            TransformOperation::ConditionalSet {
                field: "priority".to_string(),
                condition: "level=\"error\"".to_string(),
                value: "high".to_string(),
            },
            // Enrich with user information
            TransformOperation::Enrichment {
                source: "user_service".to_string(),
                fields: vec!["user_role".to_string(), "user_team".to_string()],
            },
        ],
        continue_on_error: true,
        max_transformations_per_record: 50,
        enable_caching: true,
        cache_size_limit: 1000,
    };
    let transform_processor = Arc::new(TransformProcessor::new(transform_config));

    // Create advanced aggregate processor with multiple aggregations
    let aggregate_config = AggregateProcessorConfig {
        aggregations: vec![
            // Count records by service
            AggregationOperation::Count {
                group_by: "service".to_string(),
                output_field: "service_request_count".to_string(),
            },
            // Count records by environment
            AggregationOperation::Count {
                group_by: "environment".to_string(),
                output_field: "environment_request_count".to_string(),
            },
            // Average response times by service
            AggregationOperation::Average {
                field: "response_time_ms".to_string(),
                group_by: Some("service".to_string()),
                output_field: "avg_response_time".to_string(),
            },
            // Maximum response times by service
            AggregationOperation::Max {
                field: "response_time_ms".to_string(),
                group_by: Some("service".to_string()),
                output_field: "max_response_time".to_string(),
            },
            // 95th percentile response times by service
            AggregationOperation::Percentile {
                field: "response_time_ms".to_string(),
                percentile: 95.0,
                group_by: Some("service".to_string()),
                output_field: "p95_response_time".to_string(),
            },
            // Request rate per minute globally
            AggregationOperation::Rate {
                time_window_seconds: 60,
                group_by: None,
                output_field: "global_request_rate".to_string(),
            },
        ],
        time_window: TimeWindow::Fixed {
            duration_seconds: 30,
        },
        emit_intermediate: true,
        max_groups: 1000,
        reset_after_emit: true,
    };
    let aggregate_processor = Arc::new(AggregateProcessor::new(aggregate_config));

    // Create mock exporter for output
    let mock_exporter_config = MockExporterConfig {
        simulate_delay: true,
        export_delay_ms: 10,
        error_probability: 0.0,
        destination: "analytics_db".to_string(),
        backpressure_threshold: 1000,
        simulate_backpressure: false,
        simulate_errors: false,
    };
    let mock_exporter = Arc::new(MockExporter::new(mock_exporter_config));
    println!("Created advanced processors and exporter");

    // Create pipeline with advanced processing
    let pipeline_config = bridge_core::pipeline::PipelineConfig {
        name: "advanced_processors_pipeline".to_string(),
        max_batch_size: 20,
        flush_interval_ms: 5000,
        buffer_size: 1000,
        enable_backpressure: true,
        backpressure_threshold: 80,
        enable_metrics: true,
        enable_health_checks: true,
        health_check_interval_ms: 1000,
    };

    let mut pipeline = TelemetryIngestionPipeline::new(pipeline_config);

    // Add components to pipeline
    pipeline.add_receiver(mock_receiver.clone());
    pipeline.add_processor(transform_processor.clone());
    pipeline.add_processor(aggregate_processor.clone());
    pipeline.add_exporter(mock_exporter.clone());

    let pipeline = Arc::new(RwLock::new(pipeline));
    println!("Created pipeline with advanced processing chain");

    // Add health checks
    bridge_system
        .health_checker
        .add_health_check(mock_receiver.clone())
        .await?;
    bridge_system
        .health_checker
        .add_health_check(transform_processor.clone())
        .await?;
    bridge_system
        .health_checker
        .add_health_check(aggregate_processor.clone())
        .await?;
    bridge_system
        .health_checker
        .add_health_check(mock_exporter.clone())
        .await?;
    println!("Added health checks");

    // Start components
    mock_receiver.start().await?;
    transform_processor.start().await?;
    aggregate_processor.start().await?;
    mock_exporter.start().await?;
    println!("Started all components");

    // Set and start pipeline
    bridge_system.set_pipeline(pipeline.clone()).await?;
    bridge_system.start_pipeline().await?;
    println!("Started pipeline");

    // Process data for a longer period to see aggregations
    println!("Processing data for 60 seconds to demonstrate aggregations...");
    for i in 1..=12 {
        sleep(Duration::from_secs(5)).await;
        println!("Processing... {} seconds elapsed", i * 5);

        // Show intermediate statistics every 15 seconds
        if i % 3 == 0 {
            let transform_stats = transform_processor.get_stats().await?;
            let aggregate_stats = aggregate_processor.get_stats().await?;
            let exporter_stats = mock_exporter.get_stats().await?;

            println!("=== Intermediate Statistics ({}s) ===", i * 5);
            println!("Transform Processor:");
            println!("  - Records processed: {}", transform_stats.total_records);
            println!(
                "  - Avg processing time: {:.2}ms",
                transform_stats.avg_processing_time_ms
            );
            println!("Aggregate Processor:");
            println!("  - Records processed: {}", aggregate_stats.total_records);
            println!(
                "  - Avg processing time: {:.2}ms",
                aggregate_stats.avg_processing_time_ms
            );
            println!("Mock Exporter:");
            println!("  - Batches exported: {}", exporter_stats.total_batches);
            println!("  - Records exported: {}", exporter_stats.total_records);
            println!();
        }
    }

    // Get final status and statistics
    let status = bridge_system.get_status().await;
    println!("=== Final Bridge Status ===");
    println!("{:?}", status);

    println!("\n=== Final Component Statistics ===");

    let receiver_stats = mock_receiver.get_stats().await?;
    println!("Mock Receiver:");
    println!(
        "  - Total records generated: {}",
        receiver_stats.total_records
    );
    println!("  - Error count: {}", receiver_stats.error_count);

    let transform_stats = transform_processor.get_stats().await?;
    println!("Transform Processor:");
    println!(
        "  - Total records processed: {}",
        transform_stats.total_records
    );
    println!("  - Error count: {}", transform_stats.error_count);
    println!(
        "  - Average processing time: {:.2}ms",
        transform_stats.avg_processing_time_ms
    );

    let aggregate_stats = aggregate_processor.get_stats().await?;
    println!("Aggregate Processor:");
    println!(
        "  - Total records processed: {}",
        aggregate_stats.total_records
    );
    println!("  - Error count: {}", aggregate_stats.error_count);
    println!(
        "  - Average processing time: {:.2}ms",
        aggregate_stats.avg_processing_time_ms
    );

    let exporter_stats = mock_exporter.get_stats().await?;
    println!("Mock Exporter:");
    println!(
        "  - Total batches exported: {}",
        exporter_stats.total_batches
    );
    println!(
        "  - Total records exported: {}",
        exporter_stats.total_records
    );
    println!(
        "  - Average export time: {:.2}ms",
        exporter_stats.avg_export_time_ms
    );

    // Demonstrate processor capabilities
    println!("\n=== Advanced Processing Capabilities Demonstrated ===");
    println!("🔄 Transform Processor:");
    println!("  ✅ Field mapping (service.name → service)");
    println!("  ✅ Attribute addition (environment, processed_at)");
    println!("  ✅ Conditional attribute setting (priority based on level)");
    println!("  ✅ JSON extraction (region from metadata)");
    println!("  ✅ Data enrichment (user information)");
    println!("  ✅ Error handling and continuation");

    println!("\n📊 Aggregate Processor:");
    println!("  ✅ Count aggregations by service and environment");
    println!("  ✅ Average, max, and percentile calculations");
    println!("  ✅ Rate calculations (requests per minute)");
    println!("  ✅ Time window-based aggregations");
    println!("  ✅ Group-based metrics generation");

    // Stop pipeline and components
    bridge_system.stop_pipeline().await?;
    println!("\nStopped pipeline");

    mock_receiver.stop().await?;
    transform_processor.stop().await?;
    aggregate_processor.stop().await?;
    mock_exporter.stop().await?;
    println!("Stopped all components");

    println!("\n🎉 Advanced processors example completed successfully!");
    println!("The Transform and Aggregate processors demonstrated sophisticated");
    println!("telemetry data processing capabilities for production use cases.");

    Ok(())
}
