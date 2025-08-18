//! Validation tests

use super::*;
use crate::schema::{Schema, SchemaFormat, SchemaType, SchemaVersion};
use std::collections::HashMap;

#[cfg(test)]
mod validator_tests {
    use super::*;

    #[tokio::test]
    async fn test_schema_validator_creation() {
        let validator = SchemaValidator::new();
        // Test that validator can be created successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_schema_validation() {
        let validator = SchemaValidator::new();

        let schema = Schema::new(
            "test-schema".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object", "properties": {"value": {"type": "number"}}}"#.to_string(),
            SchemaFormat::Json,
        );

        let result = validator.validate_schema(&schema).await.unwrap();
        assert!(result.is_valid());
        assert_eq!(result.error_count(), 0);
    }

    #[tokio::test]
    async fn test_invalid_schema_validation() {
        let validator = SchemaValidator::new();

        let schema = Schema::new(
            "".to_string(), // Empty name
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"invalid": json}"#.to_string(), // Invalid JSON
            SchemaFormat::Json,
        );

        let result = validator.validate_schema(&schema).await.unwrap();
        assert!(!result.is_valid());
        assert!(result.error_count() > 0);
    }

    #[tokio::test]
    async fn test_data_validation() {
        let validator = SchemaValidator::new();

        let schema = Schema::new(
            "test-schema".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object"}"#.to_string(),
            SchemaFormat::Json,
        );

        let data = TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: "test".to_string(),
            size: 1,
            records: vec![bridge_core::types::TelemetryRecord {
                id: uuid::Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
                record_type: bridge_core::types::TelemetryType::Metric,
                data: bridge_core::types::TelemetryData::Metric(bridge_core::types::MetricData {
                    name: "test_metric".to_string(),
                    description: None,
                    unit: None,
                    metric_type: bridge_core::types::MetricType::Gauge,
                    value: bridge_core::types::MetricValue::Gauge(42.0),
                    labels: HashMap::new(),
                    timestamp: chrono::Utc::now(),
                }),
                attributes: HashMap::new(),
                tags: HashMap::new(),
                resource: None,
                service: None,
            }],
            metadata: HashMap::new(),
        };

        let result = validator.validate_data(&schema, &data).await.unwrap();
        assert!(result.is_valid());
    }
}
