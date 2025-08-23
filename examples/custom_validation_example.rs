//! Custom validation example for the enrichment processor
//!
//! This example demonstrates how to use JSONPath-based custom validation
//! with the enrichment processor.

use bridge_core::types::{
    MetricData, MetricType, MetricValue, TelemetryBatch, TelemetryData, TelemetryRecord,
    TelemetryType,
};
use bridge_core::TelemetryProcessor;
use chrono::Utc;
use ingestion::processors::{
    enrichment_processor::{
        EnrichmentProcessor, EnrichmentProcessorConfig, ValidationFailureAction, ValidationRule,
        ValidationType,
    },
    ProcessorConfig,
};
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create enrichment processor configuration
    let mut config = EnrichmentProcessorConfig::new();

    // Add custom validation rules
    config.add_validation_rule(ValidationRule {
        name: "service_name_required".to_string(),
        description: Some("Ensure service name is present and valid".to_string()),
        field: "attributes.service_name".to_string(),
        validation_type: ValidationType::Custom(
            "$.attributes.service_name exists && $.attributes.service_name != \"\"".to_string(),
        ),
        parameters: HashMap::new(),
        failure_action: ValidationFailureAction::Drop,
        enabled: true,
    });

    config.add_validation_rule(ValidationRule {
        name: "email_validation".to_string(),
        description: Some("Validate email format for user_email field".to_string()),
        field: "attributes.user_email".to_string(),
        validation_type: ValidationType::Custom("$.attributes.user_email is_email()".to_string()),
        parameters: HashMap::new(),
        failure_action: ValidationFailureAction::MarkError,
        enabled: true,
    });

    config.add_validation_rule(ValidationRule {
        name: "response_time_range".to_string(),
        description: Some("Ensure response time is within reasonable range".to_string()),
        field: "attributes.response_time".to_string(),
        validation_type: ValidationType::Custom("$.attributes.response_time is_number() && $.attributes.response_time >= 0 && $.attributes.response_time <= 30000".to_string()),
        parameters: HashMap::new(),
        failure_action: ValidationFailureAction::UseDefault("1000".to_string()),
        enabled: true,
    });

    config.add_validation_rule(ValidationRule {
        name: "production_environment_validation".to_string(),
        description: Some("Validate production environment settings".to_string()),
        field: "attributes.environment".to_string(),
        validation_type: ValidationType::Custom("$.attributes.environment == \"production\" && $.attributes.service_name == \"api-gateway\" && $.tags.version starts_with \"1.\"".to_string()),
        parameters: HashMap::new(),
        failure_action: ValidationFailureAction::SkipEnrichment,
        enabled: true,
    });

    // Create the enrichment processor
    let processor = EnrichmentProcessor::new(&config as &dyn ProcessorConfig).await?;

    // Create test telemetry records
    let mut valid_record = TelemetryRecord {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        record_type: TelemetryType::Metric,
        data: TelemetryData::Metric(MetricData {
            name: "http_requests_total".to_string(),
            description: Some("Total HTTP requests".to_string()),
            unit: Some("count".to_string()),
            metric_type: MetricType::Counter,
            value: MetricValue::Counter(100.0),
            labels: HashMap::new(),
            timestamp: Utc::now(),
        }),
        attributes: {
            let mut attrs = HashMap::new();
            attrs.insert("service_name".to_string(), "api-gateway".to_string());
            attrs.insert("environment".to_string(), "production".to_string());
            attrs.insert("user_email".to_string(), "user@example.com".to_string());
            attrs.insert("response_time".to_string(), "150.5".to_string());
            attrs
        },
        tags: {
            let mut tags = HashMap::new();
            tags.insert("version".to_string(), "1.0.0".to_string());
            tags.insert("region".to_string(), "us-west-2".to_string());
            tags
        },
        resource: None,
        service: None,
    };

    let mut invalid_record = TelemetryRecord {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        record_type: TelemetryType::Metric,
        data: TelemetryData::Metric(MetricData {
            name: "http_requests_total".to_string(),
            description: Some("Total HTTP requests".to_string()),
            unit: Some("count".to_string()),
            metric_type: MetricType::Counter,
            value: MetricValue::Counter(100.0),
            labels: HashMap::new(),
            timestamp: Utc::now(),
        }),
        attributes: {
            let mut attrs = HashMap::new();
            attrs.insert("service_name".to_string(), "".to_string()); // Invalid: empty service name
            attrs.insert("environment".to_string(), "production".to_string());
            attrs.insert("user_email".to_string(), "invalid-email".to_string()); // Invalid: not an email
            attrs.insert("response_time".to_string(), "50000".to_string()); // Invalid: too high
            attrs
        },
        tags: {
            let mut tags = HashMap::new();
            tags.insert("version".to_string(), "2.0.0".to_string()); // Invalid: doesn't start with "1."
            tags.insert("region".to_string(), "us-west-2".to_string());
            tags
        },
        resource: None,
        service: None,
    };

    // Create telemetry batch
    let batch = TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        source: "example".to_string(),
        size: 2,
        records: vec![valid_record, invalid_record],
        metadata: HashMap::new(),
    };

    println!("Processing telemetry batch with custom validation...");
    println!("Batch contains {} records", batch.records.len());

    // Process the batch
    match processor.process(batch).await {
        Ok(processed_batch) => {
            println!("Processing completed successfully!");
            println!("Status: {:?}", processed_batch.status);
            println!("Processed {} records", processed_batch.records.len());

            for (i, record) in processed_batch.records.iter().enumerate() {
                println!("Record {}: {:?}", i, record.status);
                if !record.errors.is_empty() {
                    println!("  Errors: {:?}", record.errors);
                }
            }
        }
        Err(e) => {
            eprintln!("Processing failed: {}", e);
        }
    }

    Ok(())
}
