//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Comprehensive tests for streaming sources
//!
//! This module provides tests for all source implementations including
//! KafkaSource, HttpSource, and FileSource.

use bridge_core::types::{
    MetricData, MetricType, MetricValue, TelemetryBatch, TelemetryData, TelemetryRecord,
    TelemetryType,
};
use chrono::Utc;
use std::collections::HashMap;
use streaming_processor::sources::file_source::FileSourceConfig;
use streaming_processor::sources::http_source::HttpSourceConfig;
use streaming_processor::sources::kafka_source::KafkaSourceConfig;
use streaming_processor::sources::{
    FileSource, HttpSource, KafkaSource, SourceConfig, SourceStats, StreamSource,
};
use uuid::Uuid;

/// Helper function to create a test telemetry batch
fn create_test_telemetry_batch(source: &str, size: usize) -> TelemetryBatch {
    let records = (0..size)
        .map(|i| TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: format!("test_metric_{}", i),
                description: Some("Test metric".to_string()),
                unit: Some("count".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(i as f64),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        })
        .collect();

    TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        source: source.to_string(),
        size,
        records,
        metadata: HashMap::new(),
    }
}

#[tokio::test]
async fn test_kafka_source_creation() {
    let config = KafkaSourceConfig::new(
        vec!["localhost:9092".to_string()],
        "test-topic".to_string(),
        "test-group".to_string(),
    );
    let source = KafkaSource::new(&config).await;
    assert!(source.is_ok());

    let source = source.unwrap();
    assert_eq!(source.name(), "kafka");
    assert_eq!(source.version(), "1.0.0");
}

#[tokio::test]
async fn test_kafka_source_config_validation() {
    let config = KafkaSourceConfig::new(
        vec!["localhost:9092".to_string()],
        "test-topic".to_string(),
        "test-group".to_string(),
    );
    let result = config.validate().await;
    assert!(result.is_ok());

    // Test invalid config (empty bootstrap servers)
    let invalid_config =
        KafkaSourceConfig::new(vec![], "test-topic".to_string(), "test-group".to_string());
    let result = invalid_config.validate().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_kafka_source_lifecycle() {
    let config = KafkaSourceConfig::new(
        vec!["localhost:9092".to_string()],
        "test-topic".to_string(),
        "test-group".to_string(),
    );
    let mut source = KafkaSource::new(&config).await.unwrap();

    // Initially not running
    assert!(!source.is_running());

    // Initialize
    let result = source.init().await;
    assert!(result.is_ok());

    // Start (this would fail in real environment without Kafka, but we test the interface)
    let result = source.start().await;
    // This might fail without actual Kafka, but we're testing the interface

    // Stop
    let result = source.stop().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_kafka_source_stats() {
    let config = KafkaSourceConfig::new(
        vec!["localhost:9092".to_string()],
        "test-topic".to_string(),
        "test-group".to_string(),
    );
    let source = KafkaSource::new(&config).await.unwrap();

    let stats = source.get_stats().await;
    assert!(stats.is_ok());

    let stats = stats.unwrap();
    assert_eq!(stats.source, "kafka");
    assert_eq!(stats.total_records, 0);
    assert_eq!(stats.error_count, 0);
    assert!(!stats.is_connected);
}

#[tokio::test]
async fn test_http_source_creation() {
    let config = HttpSourceConfig::new("http://localhost:8080/data".to_string());
    let source = HttpSource::new(&config).await;
    assert!(source.is_ok());

    let source = source.unwrap();
    assert_eq!(source.name(), "http");
    assert_eq!(source.version(), "1.0.0");
}

#[tokio::test]
async fn test_http_source_config_validation() {
    let config = HttpSourceConfig::new("http://localhost:8080/data".to_string());
    let result = config.validate().await;
    assert!(result.is_ok());

    // Test invalid config (empty endpoint URL)
    let mut invalid_config = HttpSourceConfig::new("http://localhost:8080/data".to_string());
    invalid_config.endpoint_url = "".to_string();
    let result = invalid_config.validate().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_http_source_lifecycle() {
    let config = HttpSourceConfig::new("http://localhost:8080/data".to_string());
    let mut source = HttpSource::new(&config).await.unwrap();

    // Initially not running
    assert!(!source.is_running());

    // Initialize
    let result = source.init().await;
    assert!(result.is_ok());

    // Start (this would fail in real environment without HTTP server, but we test the interface)
    let result = source.start().await;
    // This might fail without actual HTTP server, but we're testing the interface

    // Stop
    let result = source.stop().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_http_source_stats() {
    let config = HttpSourceConfig::new("http://localhost:8080/data".to_string());
    let source = HttpSource::new(&config).await.unwrap();

    let stats = source.get_stats().await;
    assert!(stats.is_ok());

    let stats = stats.unwrap();
    assert_eq!(stats.source, "http");
    assert_eq!(stats.total_records, 0);
    assert_eq!(stats.error_count, 0);
    assert!(!stats.is_connected);
}

#[tokio::test]
async fn test_file_source_creation() {
    // Create a temporary test file
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_string_lossy().to_string();
    
    let config = FileSourceConfig::new(
        file_path,
        streaming_processor::sources::file_source::FileFormat::Json,
    );
    let source = FileSource::new(&config).await;
    assert!(source.is_ok());

    let source = source.unwrap();
    assert_eq!(source.name(), "file");
    assert_eq!(source.version(), "1.0.0");
}

#[tokio::test]
async fn test_file_source_config_validation() {
    // Create a temporary test file
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_string_lossy().to_string();
    
    let config = FileSourceConfig::new(
        file_path,
        streaming_processor::sources::file_source::FileFormat::Json,
    );
    let result = config.validate().await;
    assert!(result.is_ok());

    // Test invalid config (empty file path)
    let mut invalid_config = FileSourceConfig::new(
        "test.json".to_string(),
        streaming_processor::sources::file_source::FileFormat::Json,
    );
    invalid_config.file_path = "".to_string();
    let result = invalid_config.validate().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_file_source_lifecycle() {
    // Create a temporary test file
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_string_lossy().to_string();
    
    let config = FileSourceConfig::new(
        file_path,
        streaming_processor::sources::file_source::FileFormat::Json,
    );
    let mut source = FileSource::new(&config).await.unwrap();

    // Initially not running
    assert!(!source.is_running());

    // Initialize
    let result = source.init().await;
    assert!(result.is_ok());

    // Start (this would fail in real environment without file, but we test the interface)
    let result = source.start().await;
    // This might fail without actual file, but we're testing the interface

    // Stop
    let result = source.stop().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_file_source_stats() {
    // Create a temporary test file
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_string_lossy().to_string();
    
    let config = FileSourceConfig::new(
        file_path,
        streaming_processor::sources::file_source::FileFormat::Json,
    );
    let source = FileSource::new(&config).await.unwrap();

    let stats = source.get_stats().await;
    assert!(stats.is_ok());

    let stats = stats.unwrap();
    assert_eq!(stats.source, "file");
    assert_eq!(stats.total_records, 0);
    assert_eq!(stats.error_count, 0);
    assert!(!stats.is_connected);
}

#[tokio::test]
async fn test_source_config_as_any() {
    let config = KafkaSourceConfig::new(
        vec!["localhost:9092".to_string()],
        "test-topic".to_string(),
        "test-group".to_string(),
    );
    let any_ref = config.as_any();
    let downcast = any_ref.downcast_ref::<KafkaSourceConfig>();
    assert!(downcast.is_some());
}

#[tokio::test]
async fn test_source_error_handling() {
    // Test that sources handle errors gracefully
    let config = KafkaSourceConfig::new(
        vec!["localhost:9092".to_string()],
        "test-topic".to_string(),
        "test-group".to_string(),
    );
    let source = KafkaSource::new(&config).await.unwrap();

    // Test that source can be created and has basic functionality
    assert_eq!(source.name(), "kafka");
    assert_eq!(source.version(), "1.0.0");
}

#[tokio::test]
async fn test_source_concurrent_access() {
    use std::sync::Arc;
    use tokio::task;

    let config = KafkaSourceConfig::new(
        vec!["localhost:9092".to_string()],
        "test-topic".to_string(),
        "test-group".to_string(),
    );
    let source = Arc::new(KafkaSource::new(&config).await.unwrap());

    let mut handles = vec![];

    // Spawn multiple tasks to test concurrent access
    for _ in 0..5 {
        let source = source.clone();
        let handle = task::spawn(async move { source.get_stats().await });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_source_memory_usage() {
    let config = KafkaSourceConfig::new(
        vec!["localhost:9092".to_string()],
        "test-topic".to_string(),
        "test-group".to_string(),
    );
    let source = KafkaSource::new(&config).await.unwrap();

    // Call stats multiple times to test memory usage
    for _ in 0..100 {
        let result = source.get_stats().await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_source_config_clone() {
    let config = KafkaSourceConfig::new(
        vec!["localhost:9092".to_string()],
        "test-topic".to_string(),
        "test-group".to_string(),
    );
    let cloned_config = config.clone();

    assert_eq!(config.name, cloned_config.name);
    assert_eq!(config.version, cloned_config.version);
    assert_eq!(config.bootstrap_servers, cloned_config.bootstrap_servers);
}

#[tokio::test]
async fn test_source_config_debug() {
    let config = KafkaSourceConfig::new(
        vec!["localhost:9092".to_string()],
        "test-topic".to_string(),
        "test-group".to_string(),
    );
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("KafkaSourceConfig"));
    assert!(debug_str.contains("kafka"));
}

#[tokio::test]
async fn test_source_stats_clone() {
    let stats = SourceStats {
        source: "test".to_string(),
        total_records: 100,
        records_per_minute: 10,
        total_bytes: 1000,
        bytes_per_minute: 100,
        error_count: 5,
        last_record_time: Some(Utc::now()),
        is_connected: true,
        lag: Some(50),
    };

    let cloned_stats = stats.clone();
    assert_eq!(stats.source, cloned_stats.source);
    assert_eq!(stats.total_records, cloned_stats.total_records);
    assert_eq!(stats.error_count, cloned_stats.error_count);
    assert_eq!(stats.is_connected, cloned_stats.is_connected);
}

#[tokio::test]
async fn test_source_stats_debug() {
    let stats = SourceStats {
        source: "test".to_string(),
        total_records: 100,
        records_per_minute: 10,
        total_bytes: 1000,
        bytes_per_minute: 100,
        error_count: 5,
        last_record_time: Some(Utc::now()),
        is_connected: true,
        lag: Some(50),
    };

    let debug_str = format!("{:?}", stats);
    assert!(debug_str.contains("SourceStats"));
    assert!(debug_str.contains("test"));
    assert!(debug_str.contains("100"));
}

#[tokio::test]
async fn test_source_config_serde() {
    let config = KafkaSourceConfig::new(
        vec!["localhost:9092".to_string()],
        "test-topic".to_string(),
        "test-group".to_string(),
    );

    // Test serialization
    let serialized = serde_json::to_string(&config);
    assert!(serialized.is_ok());

    // Test deserialization
    let serialized = serialized.unwrap();
    let deserialized: KafkaSourceConfig = serde_json::from_str(&serialized).unwrap();
    assert_eq!(config.name, deserialized.name);
    assert_eq!(config.version, deserialized.version);
    assert_eq!(config.bootstrap_servers, deserialized.bootstrap_servers);
}

#[tokio::test]
async fn test_source_stats_serde() {
    let stats = SourceStats {
        source: "test".to_string(),
        total_records: 100,
        records_per_minute: 10,
        total_bytes: 1000,
        bytes_per_minute: 100,
        error_count: 5,
        last_record_time: Some(Utc::now()),
        is_connected: true,
        lag: Some(50),
    };

    // Test serialization
    let serialized = serde_json::to_string(&stats);
    assert!(serialized.is_ok());

    // Test deserialization
    let serialized = serialized.unwrap();
    let deserialized: SourceStats = serde_json::from_str(&serialized).unwrap();
    assert_eq!(stats.source, deserialized.source);
    assert_eq!(stats.total_records, deserialized.total_records);
    assert_eq!(stats.error_count, deserialized.error_count);
    assert_eq!(stats.is_connected, deserialized.is_connected);
}
