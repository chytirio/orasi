//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry data conversion utilities
//!
//! This module provides utilities for converting between different telemetry
//! data formats and protocols.

pub mod arrow;
pub mod converter;
pub mod json;
pub mod otlp;
pub mod validator;

// Re-export main types and traits
pub use converter::{ConversionFactory, TelemetryConverter};
pub use validator::DataValidator;

// Re-export converter implementations
pub use arrow::ArrowConverter;
pub use json::JsonConverter;
pub use otlp::OtlpConverter;

#[cfg(test)]
mod test_module {
    use super::*;
    use bridge_core::types::{LogLevel, SpanKind, StatusCode};
    use chrono::{DateTime, Utc};
    use std::collections::HashMap;
    use uuid::Uuid;

    fn create_test_telemetry_batch() -> bridge_core::TelemetryBatch {
        let records = vec![
            bridge_core::types::TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: bridge_core::types::TelemetryType::Metric,
                data: bridge_core::types::TelemetryData::Metric(bridge_core::types::MetricData {
                    name: "test_metric".to_string(),
                    description: Some("Test metric".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: bridge_core::types::MetricType::Gauge,
                    value: bridge_core::types::MetricValue::Gauge(42.0),
                    labels: HashMap::from([
                        ("label1".to_string(), "value1".to_string()),
                        ("label2".to_string(), "value2".to_string()),
                    ]),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::from([("attr1".to_string(), "value1".to_string())]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            },
            bridge_core::types::TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: bridge_core::types::TelemetryType::Log,
                data: bridge_core::types::TelemetryData::Log(bridge_core::types::LogData {
                    timestamp: Utc::now(),
                    level: LogLevel::Info,
                    message: "Test log message".to_string(),
                    attributes: HashMap::from([("log_attr".to_string(), "log_value".to_string())]),
                    body: Some("Test log body".to_string()),
                    severity_number: Some(9),
                    severity_text: Some("INFO".to_string()),
                }),
                attributes: HashMap::from([("log_attr".to_string(), "log_value".to_string())]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            },
        ];

        bridge_core::TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "test".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::from([("test_key".to_string(), "test_value".to_string())]),
        }
    }

    #[tokio::test]
    async fn test_otlp_converter_creation() {
        let converter = OtlpConverter::new();
        assert_eq!(
            converter.supported_source_formats(),
            vec!["otlp", "internal"]
        );
        assert_eq!(
            converter.supported_target_formats(),
            vec!["otlp", "internal"]
        );
    }

    #[tokio::test]
    async fn test_arrow_converter_creation() {
        let converter = ArrowConverter::new();
        assert_eq!(
            converter.supported_source_formats(),
            vec!["arrow", "internal"]
        );
        assert_eq!(
            converter.supported_target_formats(),
            vec!["arrow", "internal"]
        );
    }

    #[tokio::test]
    async fn test_json_converter_creation() {
        let converter = JsonConverter::new();
        assert_eq!(
            converter.supported_source_formats(),
            vec!["json", "internal"]
        );
        assert_eq!(
            converter.supported_target_formats(),
            vec!["json", "internal"]
        );
    }

    #[tokio::test]
    async fn test_json_converter_roundtrip() {
        let converter = JsonConverter::new();
        let batch = create_test_telemetry_batch();

        // Convert to JSON
        let json_data = converter.convert_to_json(&batch).await.unwrap();
        assert!(!json_data.is_empty());

        // Convert back from JSON
        let converted_batch = converter.convert_from_json(&json_data).await.unwrap();
        assert_eq!(converted_batch.records.len(), batch.records.len());
        assert_eq!(converted_batch.source, batch.source);
    }

    #[tokio::test]
    async fn test_conversion_factory() {
        // Test supported formats
        let formats = ConversionFactory::get_supported_formats();
        assert!(formats.contains(&"otlp".to_string()));
        assert!(formats.contains(&"arrow".to_string()));
        assert!(formats.contains(&"json".to_string()));

        // Test converter creation
        assert!(ConversionFactory::create_converter("otlp").is_some());
        assert!(ConversionFactory::create_converter("arrow").is_some());
        assert!(ConversionFactory::create_converter("json").is_some());
        assert!(ConversionFactory::create_converter("unsupported").is_none());
    }

    #[tokio::test]
    async fn test_otlp_converter_unsupported_conversion() {
        let converter = OtlpConverter::new();
        let result = converter.convert(b"test", "otlp", "unsupported").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_arrow_converter_unsupported_conversion() {
        let converter = ArrowConverter::new();
        let result = converter.convert(b"test", "arrow", "unsupported").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_json_converter_unsupported_conversion() {
        let converter = JsonConverter::new();
        let result = converter.convert(b"test", "json", "unsupported").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_otlp_converter_internal_roundtrip() {
        let converter = OtlpConverter::new();
        let batch = create_test_telemetry_batch();

        // Convert to internal format (JSON)
        let internal_data = converter.convert_to_internal(&batch).await.unwrap();
        assert!(!internal_data.is_empty());

        // Convert back from internal format
        let converted_batch = converter
            .convert_from_internal(&internal_data)
            .await
            .unwrap();
        assert_eq!(converted_batch.records.len(), batch.records.len());
        assert_eq!(converted_batch.source, batch.source);
    }

    #[tokio::test]
    async fn test_arrow_converter_internal_roundtrip() {
        let converter = ArrowConverter::new();
        let batch = create_test_telemetry_batch();

        // Convert to internal format (JSON)
        let internal_data = converter.convert_to_internal(&batch).await.unwrap();
        assert!(!internal_data.is_empty());

        // Convert back from internal format
        let converted_batch = converter
            .convert_from_internal(&internal_data)
            .await
            .unwrap();
        assert_eq!(converted_batch.records.len(), batch.records.len());
        assert_eq!(converted_batch.source, batch.source);
    }
}
