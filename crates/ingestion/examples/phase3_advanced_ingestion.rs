//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Phase 3 Advanced Ingestion Example
//!
//! This example demonstrates the advanced ingestion features implemented in Phase 3:
//! - Real OTLP conversion implementations
//! - Advanced protocol support (HTTP, gRPC, Kafka)
//! - Data processing pipeline with enrichment
//! - Real-time data transformation and validation

use ingestion::{
    conversion::{OtlpConverter, ArrowConverter, DataValidator, TelemetryConverter},
    receivers::{
        OtlpReceiver, OtlpReceiverConfig, OtlpProtocolType, OtlpProtocolConfig,
        OtlpHttpReceiver, OtlpHttpReceiverConfig,
        ReceiverFactory, ReceiverManager,
    },
    processors::{
        EnrichmentProcessor, EnrichmentProcessorConfig, EnrichmentRule, EnrichmentRuleType,
        TransformFunction, ValidationRule, ValidationType, ValidationFailureAction,
        ProcessorFactory, ProcessorPipeline,
    },
    protocols::otlp_grpc::OtlpGrpcConfig,
    protocols::otlp_arrow::OtlpArrowConfig,
};
use bridge_core::{
    types::{TelemetryBatch, TelemetryRecord, TelemetryType, TelemetryData, MetricData, MetricValue, MetricType},
    BridgeResult,
};
use chrono::Utc;
use std::collections::HashMap;
use tracing::{info, error};
use uuid::Uuid;

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("Starting Phase 3 Advanced Ingestion Example");

    // Example 1: Real OTLP Conversion Implementation
    example_otlp_conversion().await?;

    // Example 2: Advanced Protocol Support
    example_advanced_protocols().await?;

    // Example 3: Data Processing Pipeline
    example_data_processing_pipeline().await?;

    // Example 4: Real-time Data Transformation
    example_real_time_transformation().await?;

    // Example 5: Comprehensive System Integration
    example_system_integration().await?;

    info!("Phase 3 Advanced Ingestion Example completed successfully");
    Ok(())
}

/// Example 1: Real OTLP Conversion Implementation
async fn example_otlp_conversion() -> BridgeResult<()> {
    info!("=== Example 1: Real OTLP Conversion Implementation ===");

    // Create test telemetry batch
    let batch = create_test_telemetry_batch();
    info!("Created test batch with {} records", batch.records.len());

    // Test OTLP conversion
    let otlp_converter = OtlpConverter::new();
    
    // Convert to OTLP format
    let otlp_data = otlp_converter.convert_to_otlp(&batch).await?;
    info!("Converted to OTLP format: {} bytes", otlp_data.len());

    // Convert back from OTLP format
    let converted_batch = otlp_converter.convert_from_otlp(&otlp_data).await?;
    info!("Converted back from OTLP: {} records", converted_batch.records.len());

    // Test Arrow conversion
    let arrow_converter = ArrowConverter::new();
    
    // Convert to Arrow format
    let arrow_data = arrow_converter.convert_to_arrow(&batch).await?;
    info!("Converted to Arrow format: {} bytes", arrow_data.len());

    // Convert back from Arrow format
    let arrow_batch = arrow_converter.convert_from_arrow(&arrow_data).await?;
    info!("Converted back from Arrow: {} records", arrow_batch.records.len());

    // Test data validation
    let validator = DataValidator::new();
    let mut batch_to_validate = batch.clone();
    validator.validate_batch(&mut batch_to_validate).await?;
    info!("Data validation completed successfully");

    Ok(())
}

/// Example 2: Advanced Protocol Support
async fn example_advanced_protocols() -> BridgeResult<()> {
    info!("=== Example 2: Advanced Protocol Support ===");

    // Create OTLP gRPC receiver
    let grpc_config = OtlpGrpcConfig::new("127.0.0.1".to_string(), 4317);
    let otlp_config = OtlpReceiverConfig::new_grpc("127.0.0.1".to_string(), 4317);
    
    let mut grpc_receiver = OtlpReceiver::new(&otlp_config).await?;
    grpc_receiver.init().await?;
    info!("OTLP gRPC receiver initialized");

    // Create OTLP HTTP receiver
    let http_config = OtlpHttpReceiverConfig::new("127.0.0.1".to_string(), 4318);
    let mut http_receiver = OtlpHttpReceiver::new(&http_config).await?;
    http_receiver.init().await?;
    info!("OTLP HTTP receiver initialized on {}", http_receiver.get_server_addr());

    // Create receiver manager
    let mut receiver_manager = ReceiverManager::new();
    
    // Add receivers to manager
    receiver_manager.add_receiver("otlp-grpc".to_string(), Box::new(grpc_receiver));
    receiver_manager.add_receiver("otlp-http".to_string(), Box::new(http_receiver));
    
    // Start all receivers
    receiver_manager.start_all().await?;
    info!("All receivers started");

    // Check receiver health
    let health_checker = ingestion::receivers::ReceiverHealthChecker::new(
        std::sync::Arc::new(tokio::sync::RwLock::new(receiver_manager))
    );
    
    let health_status = health_checker.check_health().await?;
    for (name, healthy) in health_status {
        info!("Receiver {} health: {}", name, healthy);
    }

    Ok(())
}

/// Example 3: Data Processing Pipeline
async fn example_data_processing_pipeline() -> BridgeResult<()> {
    info!("=== Example 3: Data Processing Pipeline ===");

    // Create enrichment processor configuration
    let mut enrichment_config = EnrichmentProcessorConfig::new();
    
    // Configure service mapping
    let mut service_mapping = HashMap::new();
    service_mapping.insert("myapp".to_string(), "my-application".to_string());
    service_mapping.insert("api".to_string(), "api-service".to_string());
    enrichment_config.set_service_mapping(service_mapping);

    // Configure environment mapping
    let mut env_mapping = HashMap::new();
    env_mapping.insert("dev".to_string(), "development".to_string());
    env_mapping.insert("prod".to_string(), "production".to_string());
    enrichment_config.set_environment_mapping(env_mapping);

    // Add custom enrichment rules
    let url_rule = EnrichmentRule {
        name: "extract_domain".to_string(),
        description: Some("Extract domain from URL".to_string()),
        target_field: "domain".to_string(),
        source_expression: "url".to_string(),
        rule_type: EnrichmentRuleType::Transform(TransformFunction::ExtractDomain),
        condition: None,
        default_value: None,
        enabled: true,
    };
    enrichment_config.add_custom_rule(url_rule);

    // Add validation rules
    let required_rule = ValidationRule {
        name: "required_service".to_string(),
        description: Some("Service name is required".to_string()),
        field: "service.name".to_string(),
        validation_type: ValidationType::Required,
        parameters: HashMap::new(),
        failure_action: ValidationFailureAction::UseDefault("unknown-service".to_string()),
        enabled: true,
    };
    enrichment_config.add_validation_rule(required_rule);

    // Create processor pipeline
    let mut pipeline = ProcessorPipeline::new();
    
    // Add enrichment processor
    let enrichment_processor = EnrichmentProcessor::new(&enrichment_config).await?;
    pipeline.add_processor(Box::new(enrichment_processor));
    
    // Start pipeline
    pipeline.start().await?;
    info!("Processor pipeline started");

    // Process test batch through pipeline
    let test_batch = create_test_telemetry_batch();
    let processed_batch = pipeline.process(test_batch).await?;
    
    info!("Pipeline processing completed:");
    info!("  Status: {:?}", processed_batch.status);
    info!("  Records processed: {}", processed_batch.records.len());
    info!("  Errors: {}", processed_batch.errors.len());

    // Get pipeline statistics
    let stats = pipeline.get_stats().await?;
    for (name, stat) in stats {
        info!("Processor {} stats: {:?}", name, stat);
    }

    Ok(())
}

/// Example 4: Real-time Data Transformation
async fn example_real_time_transformation() -> BridgeResult<()> {
    info!("=== Example 4: Real-time Data Transformation ===");

    // Create advanced enrichment configuration
    let mut config = EnrichmentProcessorConfig::new();
    config.enable_timestamp_enrichment = true;
    config.enable_service_enrichment = true;
    config.enable_environment_enrichment = true;
    config.enable_host_enrichment = true;
    config.enable_custom_enrichment = true;
    config.enable_sampling = true;
    config.sampling_rate = 0.8; // 80% sampling
    config.enable_validation = true;

    // Add transformation rules
    let transform_rules = vec![
        EnrichmentRule {
            name: "uppercase_service".to_string(),
            description: Some("Convert service name to uppercase".to_string()),
            target_field: "service.name.upper".to_string(),
            source_expression: "service.name".to_string(),
            rule_type: EnrichmentRuleType::Transform(TransformFunction::ToUpper),
            condition: None,
            default_value: None,
            enabled: true,
        },
        EnrichmentRule {
            name: "hash_user_id".to_string(),
            description: Some("Hash user ID for privacy".to_string()),
            target_field: "user.id.hash".to_string(),
            source_expression: "user.id".to_string(),
            rule_type: EnrichmentRuleType::Transform(TransformFunction::Hash),
            condition: None,
            default_value: None,
            enabled: true,
        },
        EnrichmentRule {
            name: "truncate_message".to_string(),
            description: Some("Truncate long messages".to_string()),
            target_field: "message.short".to_string(),
            source_expression: "message".to_string(),
            rule_type: EnrichmentRuleType::Transform(TransformFunction::Truncate(100)),
            condition: None,
            default_value: None,
            enabled: true,
        },
    ];

    for rule in transform_rules {
        config.add_custom_rule(rule);
    }

    // Create enrichment processor
    let processor = EnrichmentProcessor::new(&config).await?;
    
    // Process multiple batches to demonstrate real-time transformation
    for i in 0..5 {
        let mut batch = create_test_telemetry_batch();
        batch.source = format!("batch-{}", i);
        
        info!("Processing batch {} with {} records", i, batch.records.len());
        
        let start_time = std::time::Instant::now();
        let processed = processor.process(batch).await?;
        let processing_time = start_time.elapsed();
        
        info!("Batch {} processed in {:?}:", i, processing_time);
        info!("  Status: {:?}", processed.status);
        info!("  Records: {}", processed.records.len());
        info!("  Errors: {}", processed.errors.len());
        
        // Show some transformed data
        if let Some(record) = processed.records.first() {
            info!("  Sample enriched fields: {:?}", record.metadata);
        }
    }

    Ok(())
}

/// Example 5: Comprehensive System Integration
async fn example_system_integration() -> BridgeResult<()> {
    info!("=== Example 5: Comprehensive System Integration ===");

    // Create complete ingestion system
    let mut system = ingestion::IngestionSystem::new();

    // Add OTLP HTTP receiver
    let http_config = OtlpHttpReceiverConfig::new("127.0.0.1".to_string(), 4319);
    let mut http_receiver = OtlpHttpReceiver::new(&http_config).await?;
    http_receiver.init().await?;
    system.add_receiver("otlp-http".to_string(), Box::new(http_receiver));

    // Add enrichment processor
    let mut enrichment_config = EnrichmentProcessorConfig::new();
    enrichment_config.enable_timestamp_enrichment = true;
    enrichment_config.enable_service_enrichment = true;
    enrichment_config.enable_host_enrichment = true;
    
    let enrichment_processor = EnrichmentProcessor::new(&enrichment_config).await?;
    system.add_processor("enrichment".to_string(), Box::new(enrichment_processor));

    // Process test data through the system
    let test_batch = create_test_telemetry_batch();
    info!("Processing batch through complete system: {} records", test_batch.records.len());
    
    let result = system.process_batch(test_batch).await;
    match result {
        Ok(_) => info!("System processing completed successfully"),
        Err(e) => error!("System processing failed: {}", e),
    }

    // Check system health
    system.health_check().await?;
    info!("System health check completed");

    // Get system statistics
    system.get_stats().await?;
    info!("System statistics collected");

    Ok(())
}

/// Create test telemetry batch
fn create_test_telemetry_batch() -> TelemetryBatch {
    let records = vec![
        TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "http_requests_total".to_string(),
                description: Some("Total HTTP requests".to_string()),
                unit: Some("requests".to_string()),
                metric_type: MetricType::Counter,
                value: MetricValue::Counter(42.0),
                labels: HashMap::from([
                    ("method".to_string(), "GET".to_string()),
                    ("status".to_string(), "200".to_string()),
                ]),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::from([
                ("service.name".to_string(), "myapp".to_string()),
                ("environment".to_string(), "dev".to_string()),
                ("url".to_string(), "https://example.com/api/users".to_string()),
                ("user.id".to_string(), "user123".to_string()),
                ("message".to_string(), "This is a very long message that should be truncated by the enrichment processor to demonstrate the truncation functionality".to_string()),
            ]),
            tags: HashMap::new(),
            resource: None,
            service: None,
        },
        TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "response_time_seconds".to_string(),
                description: Some("HTTP response time".to_string()),
                unit: Some("seconds".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(0.125),
                labels: HashMap::from([
                    ("endpoint".to_string(), "/api/users".to_string()),
                ]),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::from([
                ("service.name".to_string(), "api".to_string()),
                ("environment".to_string(), "prod".to_string()),
            ]),
            tags: HashMap::new(),
            resource: None,
            service: None,
        },
    ];

    TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        source: "test".to_string(),
        size: records.len(),
        records,
        metadata: HashMap::from([
            ("batch_type".to_string(), "test".to_string()),
            ("version".to_string(), "1.0.0".to_string()),
        ]),
    }
}
