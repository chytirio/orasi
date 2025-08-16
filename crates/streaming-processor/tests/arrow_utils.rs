//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Comprehensive tests for arrow_utils module
//! 
//! This module provides tests for Arrow IPC serialization and deserialization
//! functionality used by the streaming processor.

use streaming_processor::arrow_utils::{serialize_to_arrow_ipc, deserialize_from_arrow_ipc};
use bridge_core::{
    types::{TelemetryBatch, TelemetryRecord, TelemetryData, TelemetryType, MetricData, MetricValue, MetricType},
};
use std::collections::HashMap;
use chrono::Utc;
use uuid::Uuid;

/// Helper function to create a test telemetry batch
fn create_test_telemetry_batch(source: &str, size: usize) -> TelemetryBatch {
    let records = (0..size).map(|i| {
        TelemetryRecord {
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
        }
    }).collect();

    TelemetryBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        source: source.to_string(),
        size,
        records,
        metadata: HashMap::new(),
    }
}

#[test]
fn test_serialize_empty_batch() {
    let batch = create_test_telemetry_batch("test", 0);
    let result = serialize_to_arrow_ipc(&batch);
    assert!(result.is_ok());
    
    let serialized = result.unwrap();
    assert!(!serialized.is_empty());
}

#[test]
fn test_serialize_single_record_batch() {
    let batch = create_test_telemetry_batch("test", 1);
    let result = serialize_to_arrow_ipc(&batch);
    assert!(result.is_ok());
    
    let serialized = result.unwrap();
    assert!(!serialized.is_empty());
}

#[test]
fn test_serialize_multiple_records_batch() {
    let batch = create_test_telemetry_batch("test", 10);
    let result = serialize_to_arrow_ipc(&batch);
    assert!(result.is_ok());
    
    let serialized = result.unwrap();
    assert!(!serialized.is_empty());
}

#[test]
fn test_deserialize_empty_batch() {
    let batch = create_test_telemetry_batch("test", 0);
    let serialized = serialize_to_arrow_ipc(&batch).unwrap();
    let result = deserialize_from_arrow_ipc(&serialized);
    assert!(result.is_ok());
    
    let deserialized = result.unwrap();
    assert_eq!(deserialized.id, batch.id);
    assert_eq!(deserialized.source, batch.source);
    assert_eq!(deserialized.size, batch.size);
    assert_eq!(deserialized.records.len(), batch.records.len());
}

#[test]
fn test_deserialize_single_record_batch() {
    let batch = create_test_telemetry_batch("test", 1);
    let serialized = serialize_to_arrow_ipc(&batch).unwrap();
    let result = deserialize_from_arrow_ipc(&serialized);
    assert!(result.is_ok());
    
    let deserialized = result.unwrap();
    assert_eq!(deserialized.id, batch.id);
    assert_eq!(deserialized.source, batch.source);
    assert_eq!(deserialized.size, batch.size);
    assert_eq!(deserialized.records.len(), batch.records.len());
}

#[test]
fn test_deserialize_multiple_records_batch() {
    let batch = create_test_telemetry_batch("test", 10);
    let serialized = serialize_to_arrow_ipc(&batch).unwrap();
    let result = deserialize_from_arrow_ipc(&serialized);
    assert!(result.is_ok());
    
    let deserialized = result.unwrap();
    assert_eq!(deserialized.id, batch.id);
    assert_eq!(deserialized.source, batch.source);
    assert_eq!(deserialized.size, batch.size);
    assert_eq!(deserialized.records.len(), batch.records.len());
}

#[test]
fn test_roundtrip_serialization() {
    let original_batch = create_test_telemetry_batch("test", 5);
    
    // Serialize
    let serialized = serialize_to_arrow_ipc(&original_batch).unwrap();
    assert!(!serialized.is_empty());
    
    // Deserialize
    let deserialized = deserialize_from_arrow_ipc(&serialized).unwrap();
    
    // Verify roundtrip
    assert_eq!(original_batch.id, deserialized.id);
    assert_eq!(original_batch.source, deserialized.source);
    assert_eq!(original_batch.size, deserialized.size);
    assert_eq!(original_batch.records.len(), deserialized.records.len());
    
    // Verify metadata
    assert_eq!(original_batch.metadata.len(), deserialized.metadata.len());
}

#[test]
fn test_serialization_with_metadata() {
    let mut batch = create_test_telemetry_batch("test", 3);
    batch.metadata.insert("key1".to_string(), "value1".to_string());
    batch.metadata.insert("key2".to_string(), "value2".to_string());
    
    let serialized = serialize_to_arrow_ipc(&batch).unwrap();
    let deserialized = deserialize_from_arrow_ipc(&serialized).unwrap();
    
    assert_eq!(batch.metadata.len(), deserialized.metadata.len());
    assert_eq!(batch.metadata.get("key1"), deserialized.metadata.get("key1"));
    assert_eq!(batch.metadata.get("key2"), deserialized.metadata.get("key2"));
}

#[test]
fn test_serialization_with_attributes() {
    let mut batch = create_test_telemetry_batch("test", 2);
    batch.records[0].attributes.insert("attr1".to_string(), "value1".to_string());
    batch.records[0].attributes.insert("attr2".to_string(), "value2".to_string());
    
    let serialized = serialize_to_arrow_ipc(&batch).unwrap();
    let deserialized = deserialize_from_arrow_ipc(&serialized).unwrap();
    
    assert_eq!(batch.records[0].attributes.len(), deserialized.records[0].attributes.len());
    assert_eq!(batch.records[0].attributes.get("attr1"), deserialized.records[0].attributes.get("attr1"));
    assert_eq!(batch.records[0].attributes.get("attr2"), deserialized.records[0].attributes.get("attr2"));
}

#[test]
fn test_serialization_with_tags() {
    let mut batch = create_test_telemetry_batch("test", 2);
    batch.records[0].tags.insert("tag1".to_string(), "value1".to_string());
    batch.records[0].tags.insert("tag2".to_string(), "value2".to_string());
    
    let serialized = serialize_to_arrow_ipc(&batch).unwrap();
    let deserialized = deserialize_from_arrow_ipc(&serialized).unwrap();
    
    assert_eq!(batch.records[0].tags.len(), deserialized.records[0].tags.len());
    assert_eq!(batch.records[0].tags.get("tag1"), deserialized.records[0].tags.get("tag1"));
    assert_eq!(batch.records[0].tags.get("tag2"), deserialized.records[0].tags.get("tag2"));
}

#[test]
fn test_serialization_with_metric_labels() {
    let mut batch = create_test_telemetry_batch("test", 2);
    if let TelemetryData::Metric(ref mut metric_data) = batch.records[0].data {
        metric_data.labels.insert("label1".to_string(), "value1".to_string());
        metric_data.labels.insert("label2".to_string(), "value2".to_string());
    }
    
    let serialized = serialize_to_arrow_ipc(&batch).unwrap();
    let deserialized = deserialize_from_arrow_ipc(&serialized).unwrap();
    
    if let (TelemetryData::Metric(original_metric), TelemetryData::Metric(deserialized_metric)) = 
        (&batch.records[0].data, &deserialized.records[0].data) {
        assert_eq!(original_metric.labels.len(), deserialized_metric.labels.len());
        assert_eq!(original_metric.labels.get("label1"), deserialized_metric.labels.get("label1"));
        assert_eq!(original_metric.labels.get("label2"), deserialized_metric.labels.get("label2"));
    }
}

#[test]
fn test_serialization_different_metric_types() {
    let mut batch = create_test_telemetry_batch("test", 4);
    
    // Set different metric types
    if let TelemetryData::Metric(ref mut metric_data) = batch.records[0].data {
        metric_data.metric_type = MetricType::Counter;
        metric_data.value = MetricValue::Counter(100.0);
    }
    
    if let TelemetryData::Metric(ref mut metric_data) = batch.records[1].data {
        metric_data.metric_type = MetricType::Gauge;
        metric_data.value = MetricValue::Gauge(50.0);
    }
    
    if let TelemetryData::Metric(ref mut metric_data) = batch.records[2].data {
        metric_data.metric_type = MetricType::Histogram;
        metric_data.value = MetricValue::Histogram {
            buckets: vec![],
            sum: 200.0,
            count: 10,
        };
    }
    
    if let TelemetryData::Metric(ref mut metric_data) = batch.records[3].data {
        metric_data.metric_type = MetricType::Summary;
        metric_data.value = MetricValue::Summary {
            quantiles: vec![],
            sum: 300.0,
            count: 20,
        };
    }
    
    let serialized = serialize_to_arrow_ipc(&batch).unwrap();
    let deserialized = deserialize_from_arrow_ipc(&serialized).unwrap();
    
    assert_eq!(batch.records.len(), deserialized.records.len());
    
    // Verify metric types are preserved
    if let TelemetryData::Metric(original_metric) = &batch.records[0].data {
        if let TelemetryData::Metric(deserialized_metric) = &deserialized.records[0].data {
            assert_eq!(original_metric.metric_type, deserialized_metric.metric_type);
        }
    }
}

#[test]
fn test_serialization_large_batch() {
    let batch = create_test_telemetry_batch("test", 1000);
    let result = serialize_to_arrow_ipc(&batch);
    assert!(result.is_ok());
    
    let serialized = result.unwrap();
    assert!(!serialized.is_empty());
    
    let deserialized = deserialize_from_arrow_ipc(&serialized).unwrap();
    assert_eq!(batch.records.len(), deserialized.records.len());
}

#[test]
fn test_deserialize_invalid_data() {
    let invalid_data = vec![0, 1, 2, 3, 4, 5]; // Not valid Arrow IPC data
    let result = deserialize_from_arrow_ipc(&invalid_data);
    assert!(result.is_err());
}

#[test]
fn test_deserialize_empty_data() {
    let empty_data = vec![];
    let result = deserialize_from_arrow_ipc(&empty_data);
    assert!(result.is_err());
}

#[test]
fn test_serialization_performance() {
    let batch = create_test_telemetry_batch("test", 100);
    
    let start = std::time::Instant::now();
    let serialized = serialize_to_arrow_ipc(&batch).unwrap();
    let serialize_time = start.elapsed();
    
    let start = std::time::Instant::now();
    let _deserialized = deserialize_from_arrow_ipc(&serialized).unwrap();
    let deserialize_time = start.elapsed();
    
    // Verify that serialization/deserialization is reasonably fast
    assert!(serialize_time.as_millis() < 1000); // Less than 1 second
    assert!(deserialize_time.as_millis() < 1000); // Less than 1 second
    
    println!("Serialization time: {:?}", serialize_time);
    println!("Deserialization time: {:?}", deserialize_time);
}

#[test]
fn test_serialization_memory_usage() {
    let batch = create_test_telemetry_batch("test", 100);
    
    // Get initial memory usage (approximate)
    let initial_size = std::mem::size_of_val(&batch);
    
    let serialized = serialize_to_arrow_ipc(&batch).unwrap();
    let serialized_size = serialized.len();
    
    // Verify that serialized size is reasonable
    assert!(serialized_size > 0);
    assert!(serialized_size < initial_size * 10); // Should not be more than 10x the original size
    
    println!("Original size: {} bytes", initial_size);
    println!("Serialized size: {} bytes", serialized_size);
}

#[tokio::test]
async fn test_concurrent_serialization() {
    use std::sync::Arc;
    use tokio::task;
    
    let batch = Arc::new(create_test_telemetry_batch("test", 10));
    let mut handles = vec![];
    
    // Spawn multiple tasks to test concurrent serialization
    for _ in 0..5 {
        let batch = batch.clone();
        let handle = task::spawn(async move {
            serialize_to_arrow_ipc(&batch)
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[test]
fn test_serialization_with_null_values() {
    let mut batch = create_test_telemetry_batch("test", 2);
    
    // Set some fields to None/null values
    if let TelemetryData::Metric(ref mut metric_data) = batch.records[0].data {
        metric_data.description = None;
        metric_data.unit = None;
    }
    
    let serialized = serialize_to_arrow_ipc(&batch).unwrap();
    let deserialized = deserialize_from_arrow_ipc(&serialized).unwrap();
    
    if let TelemetryData::Metric(deserialized_metric) = &deserialized.records[0].data {
        assert!(deserialized_metric.description.is_none());
        assert!(deserialized_metric.unit.is_none());
    }
}

#[test]
fn test_serialization_with_special_characters() {
    let mut batch = create_test_telemetry_batch("test", 1);
    
    // Add special characters to various fields
    batch.source = "test-source-with-special-chars-!@#$%^&*()".to_string();
    batch.records[0].attributes.insert("special_key".to_string(), "value with spaces and !@#$%".to_string());
    
    if let TelemetryData::Metric(ref mut metric_data) = batch.records[0].data {
        metric_data.name = "metric-name-with-special-chars-!@#$%^&*()".to_string();
        metric_data.description = Some("Description with special chars: !@#$%^&*()".to_string());
    }
    
    let serialized = serialize_to_arrow_ipc(&batch).unwrap();
    let deserialized = deserialize_from_arrow_ipc(&serialized).unwrap();
    
    assert_eq!(batch.source, deserialized.source);
    assert_eq!(batch.records[0].attributes.get("special_key"), 
               deserialized.records[0].attributes.get("special_key"));
    
    if let (TelemetryData::Metric(original_metric), TelemetryData::Metric(deserialized_metric)) = 
        (&batch.records[0].data, &deserialized.records[0].data) {
        assert_eq!(original_metric.name, deserialized_metric.name);
        assert_eq!(original_metric.description, deserialized_metric.description);
    }
}

#[test]
fn test_serialization_with_unicode() {
    let mut batch = create_test_telemetry_batch("test", 1);
    
    // Add Unicode characters to various fields
    batch.source = "test-source-with-unicode-🚀🌟🎉".to_string();
    batch.records[0].attributes.insert("unicode_key".to_string(), "value with emoji 🚀🌟🎉".to_string());
    
    if let TelemetryData::Metric(ref mut metric_data) = batch.records[0].data {
        metric_data.name = "metric-name-with-unicode-🚀🌟🎉".to_string();
        metric_data.description = Some("Description with Unicode: 🚀🌟🎉".to_string());
    }
    
    let serialized = serialize_to_arrow_ipc(&batch).unwrap();
    let deserialized = deserialize_from_arrow_ipc(&serialized).unwrap();
    
    assert_eq!(batch.source, deserialized.source);
    assert_eq!(batch.records[0].attributes.get("unicode_key"), 
               deserialized.records[0].attributes.get("unicode_key"));
    
    if let (TelemetryData::Metric(original_metric), TelemetryData::Metric(deserialized_metric)) = 
        (&batch.records[0].data, &deserialized.records[0].data) {
        assert_eq!(original_metric.name, deserialized_metric.name);
        assert_eq!(original_metric.description, deserialized_metric.description);
    }
}
