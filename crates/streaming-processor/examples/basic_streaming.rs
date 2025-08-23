//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup@gmail.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Basic streaming processor example
//!
//! This example demonstrates a complete streaming pipeline with:
//! - HTTP source for data ingestion
//! - Filter processor for data filtering
//! - Transform processor for data transformation
//! - HTTP sink for data output

use bridge_core::{BridgeResult, TelemetryBatch};
use std::collections::HashMap;
use streaming_processor::{
    processors::{
        filter_processor::{FilterMode, FilterOperator, FilterProcessorConfig, FilterRule},
        transform_processor::{TransformProcessorConfig, TransformRule, TransformRuleType},
        ProcessorFactory, ProcessorPipeline,
    },
    sinks::{http_sink::HttpSinkConfig, SinkFactory, SinkManager},
    sources::{http_source::HttpSourceConfig, SourceFactory, SourceManager},
};
use tokio::time::{sleep, Duration};
use tracing::{error, info};

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting streaming processor example");

    // Create source manager
    let mut source_manager = SourceManager::new();

    // Create HTTP source configuration
    let http_source_config = HttpSourceConfig {
        name: "http".to_string(),
        version: "1.0.0".to_string(),
        endpoint_url: "https://httpbin.org/json".to_string(), // Example endpoint
        method: "GET".to_string(),
        headers: {
            let mut headers = HashMap::new();
            headers.insert(
                "User-Agent".to_string(),
                "StreamingProcessor/1.0".to_string(),
            );
            headers
        },
        body: None,
        polling_interval_ms: 5000, // Poll every 5 seconds
        request_timeout_secs: 30,
        batch_size: 100,
        buffer_size: 1000,
        auth_token: None,
        additional_config: HashMap::new(),
        max_retries: 3,
        retry_delay_ms: 1000,
        rate_limit_requests_per_second: Some(1), // Limit to 1 request per second
        response_format: streaming_processor::sources::http_source::HttpResponseFormat::Json,
        expected_content_type: None,
    };

    // Create HTTP source
    let http_source = SourceFactory::create_source(&http_source_config).await?;
    source_manager.add_source("http_source".to_string(), http_source);

    // Create processor pipeline
    let mut processor_pipeline = ProcessorPipeline::new();

    // Create filter processor configuration
    let filter_rules = vec![
        FilterRule {
            name: "exclude_test_data".to_string(),
            field: "metric.name".to_string(),
            operator: FilterOperator::NotEquals,
            value: "test_metric".to_string(),
            enabled: true,
        },
        FilterRule {
            name: "high_value_filter".to_string(),
            field: "metric.value".to_string(),
            operator: FilterOperator::GreaterThan,
            value: "10.0".to_string(),
            enabled: true,
        },
    ];

    let filter_config = FilterProcessorConfig::new(filter_rules, FilterMode::Include);
    let filter_processor = ProcessorFactory::create_processor(&filter_config).await?;
    processor_pipeline.add_processor(filter_processor);

    // Create transform processor configuration
    let transform_rules = vec![
        TransformRule {
            name: "add_timestamp".to_string(),
            rule_type: TransformRuleType::Set,
            source_field: "".to_string(),
            target_field: "processing_timestamp".to_string(),
            transform_value: Some("${timestamp}".to_string()),
            enabled: true,
        },
        TransformRule {
            name: "add_source_tag".to_string(),
            rule_type: TransformRuleType::Set,
            source_field: "".to_string(),
            target_field: "tags.source".to_string(),
            transform_value: Some("streaming_processor".to_string()),
            enabled: true,
        },
        TransformRule {
            name: "rename_metric".to_string(),
            rule_type: TransformRuleType::Copy,
            source_field: "metric.name".to_string(),
            target_field: "original_metric_name".to_string(),
            transform_value: None,
            enabled: true,
        },
    ];

    let transform_config = TransformProcessorConfig::new(transform_rules);
    let transform_processor = ProcessorFactory::create_processor(&transform_config).await?;
    processor_pipeline.add_processor(transform_processor);

    // Create sink manager
    let mut sink_manager = SinkManager::new();

    // Create HTTP sink configuration
    let http_sink_config = HttpSinkConfig {
        name: "http".to_string(),
        version: "1.0.0".to_string(),
        endpoint_url: "https://httpbin.org/post".to_string(), // Example endpoint
        method: "POST".to_string(),
        headers: {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            headers
        },
        request_timeout_secs: 30,
        retry_count: 3,
        retry_delay_ms: 1000,
        batch_size: 100,
        auth_token: None,
        additional_config: HashMap::new(),
        content_type: "application/json".to_string(),
        rate_limit_requests_per_second: Some(2), // Limit to 2 requests per second
    };

    // Create HTTP sink
    let http_sink = SinkFactory::create_sink(&http_sink_config).await?;
    sink_manager.add_sink("http_sink".to_string(), http_sink);

    // Start all components
    info!("Starting source manager");
    source_manager.start_all().await?;

    info!("Starting sink manager");
    sink_manager.start_all().await?;

    // Main processing loop
    let mut running = true;
    let mut iteration = 0;

    while running && iteration < 10 {
        iteration += 1;
        info!("Processing iteration {}", iteration);

        // Simulate data processing
        // In a real implementation, this would be driven by the source
        let sample_batch = create_sample_batch();

        // Convert TelemetryBatch to DataStream for processing
        let input_stream = bridge_core::traits::DataStream {
            stream_id: sample_batch.id.to_string(),
            data: serde_json::to_vec(&sample_batch).unwrap_or_default(),
            metadata: sample_batch.metadata.clone(),
            timestamp: sample_batch.timestamp,
        };

        // Process through pipeline
        match processor_pipeline.process_stream(input_stream).await {
            Ok(processed_stream) => {
                info!("Processed stream: {} records", sample_batch.size);

                // Convert back to TelemetryBatch for sinks
                let processed_batch = sample_batch; // In real implementation, this would be deserialized from processed_stream

                // Send to all sinks
                match sink_manager.send_to_all(processed_batch).await {
                    Ok(results) => {
                        let success_count = results.iter().filter(|r| r.is_ok()).count();
                        let error_count = results.len() - success_count;
                        info!(
                            "Sent to {} sinks ({} success, {} errors)",
                            results.len(),
                            success_count,
                            error_count
                        );
                    }
                    Err(e) => {
                        error!("Failed to send to sinks: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to process stream: {}", e);
            }
        }

        // Get statistics
        if let Ok(source_stats) = source_manager.get_stats().await {
            info!("Source stats: {:?}", source_stats);
        }

        if let Ok(processor_stats) = processor_pipeline.get_stats().await {
            info!("Processor stats: {:?}", processor_stats);
        }

        if let Ok(sink_stats) = sink_manager.get_stats().await {
            info!("Sink stats: {:?}", sink_stats);
        }

        // Wait before next iteration
        sleep(Duration::from_secs(10)).await;
    }

    // Stop all components
    info!("Stopping all components");

    source_manager.stop_all().await?;
    sink_manager.stop_all().await?;

    info!("Streaming processor example completed");
    Ok(())
}

/// Create a sample telemetry batch for demonstration
fn create_sample_batch() -> TelemetryBatch {
    use bridge_core::types::{
        MetricData, MetricType, MetricValue, TelemetryData, TelemetryRecord, TelemetryType,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    let records = vec![
        TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "cpu_usage".to_string(),
                description: Some("CPU usage percentage".to_string()),
                unit: Some("%".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(75.5),
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("host".to_string(), "server-01".to_string());
                    labels.insert("region".to_string(), "us-west-1".to_string());
                    labels
                },
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
                name: "memory_usage".to_string(),
                description: Some("Memory usage percentage".to_string()),
                unit: Some("%".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(45.2),
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("host".to_string(), "server-01".to_string());
                    labels.insert("region".to_string(), "us-west-1".to_string());
                    labels
                },
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
                name: "test_metric".to_string(), // This should be filtered out
                description: Some("Test metric".to_string()),
                unit: Some("count".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(5.0),
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
        source: "example_source".to_string(),
        size: records.len(),
        records,
        metadata: {
            let mut metadata = HashMap::new();
            metadata.insert("example".to_string(), "true".to_string());
            metadata
        },
    }
}
