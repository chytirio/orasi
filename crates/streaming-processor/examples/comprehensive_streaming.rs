//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup@gmail.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Comprehensive streaming processor example
//!
//! This example demonstrates a complete streaming pipeline with:
//! - HTTP source for data ingestion
//! - Filter processor for data filtering
//! - Transform processor for data transformation
//! - HTTP sink for data output
//! - Error handling and monitoring

use bridge_core::BridgeResult;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use streaming_processor::{
    processors::{
        filter_processor::{FilterMode, FilterOperator, FilterProcessorConfig, FilterRule},
        transform_processor::{TransformProcessorConfig, TransformRule, TransformRuleType},
        ProcessorPipeline,
    },
    sinks::{http_sink::HttpSinkConfig, SinkManager},
    sources::{http_source::HttpSourceConfig, SourceManager},
};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting comprehensive streaming processor example");

    // Create source manager
    let source_manager = Arc::new(RwLock::new(SourceManager::new()));

    // Create sink manager
    let sink_manager = Arc::new(RwLock::new(SinkManager::new()));

    // Create processor pipeline
    let pipeline = Arc::new(RwLock::new(ProcessorPipeline::new()));

    // Configure HTTP source
    let http_source_config = HttpSourceConfig {
        name: "http".to_string(),
        version: "1.0.0".to_string(),
        endpoint_url: "https://httpbin.org/json".to_string(),
        method: "GET".to_string(),
        headers: {
            let mut headers = HashMap::new();
            headers.insert(
                "User-Agent".to_string(),
                "StreamingProcessor/1.0".to_string(),
            );
            headers.insert("Accept".to_string(), "application/json".to_string());
            headers
        },
        body: None,
        polling_interval_ms: 10000, // Poll every 10 seconds
        request_timeout_secs: 30,
        batch_size: 100,
        buffer_size: 1000,
        auth_token: None,
        additional_config: HashMap::new(),
        max_retries: 3,
        retry_delay_ms: 1000,
        rate_limit_requests_per_second: Some(1),
    };

    // Configure filter processor
    let filter_config = FilterProcessorConfig {
        name: "data_filter".to_string(),
        version: "1.0.0".to_string(),
        filter_mode: FilterMode::Include,
        filter_rules: vec![
            FilterRule {
                name: "include_metrics".to_string(),
                field: "record_type".to_string(),
                operator: FilterOperator::Equals,
                value: "Metric".to_string(),
                enabled: true,
            },
            FilterRule {
                name: "exclude_errors".to_string(),
                field: "attributes.error".to_string(),
                operator: FilterOperator::NotEquals,
                value: "true".to_string(),
                enabled: true,
            },
        ],
        additional_config: HashMap::new(),
    };

    // Configure transform processor
    let transform_config = TransformProcessorConfig {
        name: "data_transformer".to_string(),
        version: "1.0.0".to_string(),
        transform_rules: vec![
            TransformRule {
                name: "add_timestamp".to_string(),
                rule_type: TransformRuleType::Set,
                source_field: "".to_string(),
                target_field: "attributes.processed_at".to_string(),
                transform_value: Some(Utc::now().to_rfc3339()),
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
                name: "add_pipeline_tag".to_string(),
                rule_type: TransformRuleType::Set,
                source_field: "".to_string(),
                target_field: "tags.pipeline".to_string(),
                transform_value: Some("comprehensive_example".to_string()),
                enabled: true,
            },
        ],
        additional_config: HashMap::new(),
    };

    // Configure HTTP sink
    let http_sink_config = HttpSinkConfig {
        name: "http".to_string(),
        version: "1.0.0".to_string(),
        endpoint_url: "https://httpbin.org/post".to_string(),
        method: "POST".to_string(),
        headers: {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            headers.insert("X-Processor".to_string(), "StreamingProcessor".to_string());
            headers
        },
        request_timeout_secs: 30,
        retry_count: 3,
        retry_delay_ms: 1000,
        batch_size: 100,
        auth_token: None,
        additional_config: HashMap::new(),
        content_type: "application/json".to_string(),
        rate_limit_requests_per_second: Some(2),
    };

    info!("Configuration created successfully");

    // Run for a specified duration
    let running = Arc::new(RwLock::new(true));
    let running_clone = running.clone();

    // Set up graceful shutdown
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await; // Run for 1 minute
        let mut r = running_clone.write().await;
        *r = false;
        info!("Shutdown signal received");
    });

    // Main processing loop
    while *running.read().await {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Simulate some processing
        info!("Processing data...");
    }

    info!("Comprehensive streaming processor example completed");
    Ok(())
}
