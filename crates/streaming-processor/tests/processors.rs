//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Comprehensive tests for streaming processors
//! 
//! This module provides tests for all processor implementations including
//! StreamProcessor, FilterProcessor, TransformProcessor, and AggregateProcessor.

use streaming_processor::processors::{
    StreamProcessor, FilterProcessor, TransformProcessor, AggregateProcessor,
    ProcessorPipeline, ProcessorConfig,
};
use streaming_processor::processors::stream_processor::StreamProcessorConfig;
use streaming_processor::processors::filter_processor::{FilterProcessorConfig, FilterRule, FilterOperator, FilterMode};
use streaming_processor::processors::transform_processor::{TransformProcessorConfig, TransformRule, TransformRuleType};
use streaming_processor::processors::aggregate_processor::{AggregateProcessorConfig, AggregationRule, AggregationFunction, AggregationFunctionType};
use bridge_core::{
    traits::{StreamProcessor as BridgeStreamProcessor, DataStream},
    types::{TelemetryRecord, TelemetryData, TelemetryType, MetricData, MetricValue, MetricType},
};
use std::collections::HashMap;
use chrono::Utc;
use uuid::Uuid;

/// Helper function to create a test data stream
fn create_test_data_stream(stream_id: &str, data: Vec<u8>) -> DataStream {
    DataStream {
        stream_id: stream_id.to_string(),
        data,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    }
}

/// Helper function to create a test telemetry record
fn create_test_telemetry_record(name: &str, value: f64) -> TelemetryRecord {
    TelemetryRecord {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        record_type: TelemetryType::Metric,
        data: TelemetryData::Metric(MetricData {
            name: name.to_string(),
            description: Some("Test metric".to_string()),
            unit: Some("count".to_string()),
            metric_type: MetricType::Gauge,
            value: MetricValue::Gauge(value),
            labels: HashMap::new(),
            timestamp: Utc::now(),
        }),
        attributes: HashMap::new(),
        tags: HashMap::new(),
        resource: None,
        service: None,
    }
}

#[tokio::test]
async fn test_stream_processor_creation() {
    let config = StreamProcessorConfig::new();
    let processor = StreamProcessor::new(&config).await;
    assert!(processor.is_ok());
    
    let processor = processor.unwrap();
    assert_eq!(processor.name(), "stream");
    assert_eq!(processor.version(), "1.0.0");
}

#[tokio::test]
async fn test_stream_processor_processing() {
    let config = StreamProcessorConfig::new();
    let processor = StreamProcessor::new(&config).await.unwrap();
    
    let input_stream = create_test_data_stream("test-stream", vec![1, 2, 3, 4, 5]);
    let result = processor.process_stream(input_stream).await;
    assert!(result.is_ok());
    
    let output_stream = result.unwrap();
    assert_eq!(output_stream.stream_id, "test-stream");
    assert!(output_stream.metadata.contains_key("processed_by"));
    assert!(output_stream.metadata.contains_key("processed_at"));
}

#[tokio::test]
async fn test_stream_processor_health_check() {
    let config = StreamProcessorConfig::new();
    let processor = StreamProcessor::new(&config).await.unwrap();
    
    let health = processor.health_check().await;
    assert!(health.is_ok());
    assert!(health.unwrap());
}

#[tokio::test]
async fn test_stream_processor_stats() {
    let config = StreamProcessorConfig::new();
    let processor = StreamProcessor::new(&config).await.unwrap();
    
    let stats = processor.get_stats().await;
    assert!(stats.is_ok());
    
    let stats = stats.unwrap();
    assert_eq!(stats.total_records, 0);
    assert_eq!(stats.error_count, 0);
}

#[tokio::test]
async fn test_stream_processor_shutdown() {
    let config = StreamProcessorConfig::new();
    let processor = StreamProcessor::new(&config).await.unwrap();
    
    let result = processor.shutdown().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_filter_processor_creation() {
    let config = FilterProcessorConfig::new(vec![], FilterMode::Include);
    let processor = FilterProcessor::new(&config).await;
    assert!(processor.is_ok());
    
    let processor = processor.unwrap();
    assert_eq!(processor.name(), "filter");
    assert_eq!(processor.version(), "1.0.0");
}

#[tokio::test]
async fn test_filter_processor_with_rules() {
    let mut config = FilterProcessorConfig::new(vec![], FilterMode::Include);
    config.filter_rules.push(FilterRule {
        name: "test_rule".to_string(),
        field: "metric.name".to_string(),
        operator: FilterOperator::Equals,
        value: "test_metric".to_string(),
        enabled: true,
    });
    
    let processor = FilterProcessor::new(&config).await.unwrap();
    let input_stream = create_test_data_stream("test-stream", vec![1, 2, 3]);
    let result = processor.process_stream(input_stream).await;
    assert!(result.is_ok());
    
    let output_stream = result.unwrap();
    assert!(output_stream.metadata.contains_key("filtered_by"));
    assert!(output_stream.metadata.contains_key("filter_rules_applied"));
}

#[tokio::test]
async fn test_filter_processor_health_check() {
    let config = FilterProcessorConfig::new(vec![], FilterMode::Include);
    let processor = FilterProcessor::new(&config).await.unwrap();
    
    let health = processor.health_check().await;
    assert!(health.is_ok());
    assert!(health.unwrap());
}

#[tokio::test]
async fn test_filter_processor_invalid_regex() {
    let mut config = FilterProcessorConfig::new(vec![], FilterMode::Include);
    config.filter_rules.push(FilterRule {
        name: "invalid_regex".to_string(),
        field: "metric.name".to_string(),
        operator: FilterOperator::Regex,
        value: "[invalid".to_string(), // Invalid regex
        enabled: true,
    });
    
    let processor = FilterProcessor::new(&config).await;
    assert!(processor.is_err());
}

#[tokio::test]
async fn test_transform_processor_creation() {
    let config = TransformProcessorConfig::new(vec![]);
    let processor = TransformProcessor::new(&config).await;
    assert!(processor.is_ok());
    
    let processor = processor.unwrap();
    assert_eq!(processor.name(), "transform");
    assert_eq!(processor.version(), "1.0.0");
}

#[tokio::test]
async fn test_transform_processor_with_rules() {
    let mut config = TransformProcessorConfig::new(vec![]);
    config.transform_rules.push(TransformRule {
        name: "test_rule".to_string(),
        rule_type: TransformRuleType::Set,
        source_field: "".to_string(),
        target_field: "transformed".to_string(),
        transform_value: Some("test_value".to_string()),
        enabled: true,
    });
    
    let processor = TransformProcessor::new(&config).await.unwrap();
    let input_stream = create_test_data_stream("test-stream", vec![1, 2, 3]);
    let result = processor.process_stream(input_stream).await;
    assert!(result.is_ok());
    
    let output_stream = result.unwrap();
    assert!(output_stream.metadata.contains_key("transformed_by"));
    assert!(output_stream.metadata.contains_key("transform_rules_applied"));
}

#[tokio::test]
async fn test_transform_processor_health_check() {
    let config = TransformProcessorConfig::new(vec![]);
    let processor = TransformProcessor::new(&config).await.unwrap();
    
    let health = processor.health_check().await;
    assert!(health.is_ok());
    assert!(health.unwrap());
}

#[tokio::test]
async fn test_aggregate_processor_creation() {
    let config = AggregateProcessorConfig::new(vec![], 60000);
    let processor = AggregateProcessor::new(&config).await;
    assert!(processor.is_ok());
    
    let processor = processor.unwrap();
    assert_eq!(processor.name(), "aggregate");
    assert_eq!(processor.version(), "1.0.0");
}

#[tokio::test]
async fn test_aggregate_processor_with_rules() {
    let mut config = AggregateProcessorConfig::new(vec![], 60000);
    config.aggregation_rules.push(AggregationRule {
        name: "test_rule".to_string(),
        group_by_fields: vec!["metric.name".to_string()],
        aggregation_functions: vec![AggregationFunction {
            name: "sum".to_string(),
            function_type: AggregationFunctionType::Sum,
            source_field: "metric.value".to_string(),
            target_field: "sum_value".to_string(),
        }],
        enabled: true,
    });
    
    let processor = AggregateProcessor::new(&config).await.unwrap();
    let input_stream = create_test_data_stream("test-stream", vec![1, 2, 3]);
    let result = processor.process_stream(input_stream).await;
    assert!(result.is_ok());
    
    let output_stream = result.unwrap();
    assert!(output_stream.metadata.contains_key("aggregated_by"));
    assert!(output_stream.metadata.contains_key("aggregation_rules_applied"));
}

#[tokio::test]
async fn test_aggregate_processor_health_check() {
    let config = AggregateProcessorConfig::new(vec![], 60000);
    let processor = AggregateProcessor::new(&config).await.unwrap();
    
    let health = processor.health_check().await;
    assert!(health.is_ok());
    assert!(health.unwrap());
}

#[tokio::test]
async fn test_processor_pipeline_creation() {
    let pipeline = ProcessorPipeline::new();
    assert_eq!(pipeline.len(), 0);
    assert!(pipeline.is_empty());
}

#[tokio::test]
async fn test_processor_pipeline_add_remove() {
    let mut pipeline = ProcessorPipeline::new();
    
    // Add processors
    let config1 = StreamProcessorConfig::new();
    let processor1 = StreamProcessor::new(&config1).await.unwrap();
    pipeline.add_processor(Box::new(processor1));
    
    let config2 = FilterProcessorConfig::new(vec![], FilterMode::Include);
    let processor2 = FilterProcessor::new(&config2).await.unwrap();
    pipeline.add_processor(Box::new(processor2));
    
    assert_eq!(pipeline.len(), 2);
    assert!(!pipeline.is_empty());
    
    // Get processor
    let processor = pipeline.get_processor(0);
    assert!(processor.is_some());
    assert_eq!(processor.unwrap().name(), "stream");
    
    // Remove processor
    let removed = pipeline.remove_processor(0);
    assert!(removed.is_some());
    assert_eq!(pipeline.len(), 1);
}

#[tokio::test]
async fn test_processor_pipeline_processing() {
    let mut pipeline = ProcessorPipeline::new();
    
    // Add a stream processor
    let config = StreamProcessorConfig::new();
    let processor = StreamProcessor::new(&config).await.unwrap();
    pipeline.add_processor(Box::new(processor));
    
    let input_stream = create_test_data_stream("test-stream", vec![1, 2, 3, 4, 5]);
    let result = pipeline.process_stream(input_stream).await;
    assert!(result.is_ok());
    
    let output_stream = result.unwrap();
    assert_eq!(output_stream.stream_id, "test-stream");
    assert!(output_stream.metadata.contains_key("processed_by"));
}

#[tokio::test]
async fn test_processor_pipeline_health_check() {
    let mut pipeline = ProcessorPipeline::new();
    
    // Add a healthy processor
    let config = StreamProcessorConfig::new();
    let processor = StreamProcessor::new(&config).await.unwrap();
    pipeline.add_processor(Box::new(processor));
    
    let health = pipeline.health_check().await;
    assert!(health.is_ok());
    assert!(health.unwrap());
}

#[tokio::test]
async fn test_processor_pipeline_stats() {
    let mut pipeline = ProcessorPipeline::new();
    
    // Add a processor
    let config = StreamProcessorConfig::new();
    let processor = StreamProcessor::new(&config).await.unwrap();
    pipeline.add_processor(Box::new(processor));
    
    let stats = pipeline.get_stats().await;
    assert!(stats.is_ok());
    
    let stats = stats.unwrap();
    assert_eq!(stats.total_records, 0);
    assert_eq!(stats.error_count, 0);
}

#[tokio::test]
async fn test_processor_pipeline_shutdown() {
    let mut pipeline = ProcessorPipeline::new();
    
    // Add a processor
    let config = StreamProcessorConfig::new();
    let processor = StreamProcessor::new(&config).await.unwrap();
    pipeline.add_processor(Box::new(processor));
    
    let result = pipeline.shutdown().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_processor_config_validation() {
    // Test valid config
    let config = StreamProcessorConfig::new();
    let result = config.validate().await;
    assert!(result.is_ok());
    
    // Test invalid config (batch_size = 0)
    let mut invalid_config = StreamProcessorConfig::new();
    invalid_config.batch_size = 0;
    let result = invalid_config.validate().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_filter_processor_config_validation() {
    // Test valid config
    let config = FilterProcessorConfig::new(vec![], FilterMode::Include);
    let result = config.validate().await;
    assert!(result.is_ok());
    
    // Test invalid config (empty name)
    let mut invalid_config = FilterProcessorConfig::new(vec![], FilterMode::Include);
    invalid_config.name = "".to_string();
    let result = invalid_config.validate().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_transform_processor_config_validation() {
    // Test valid config
    let config = TransformProcessorConfig::new(vec![]);
    let result = config.validate().await;
    assert!(result.is_ok());
    
    // Test invalid config (empty name)
    let mut invalid_config = TransformProcessorConfig::new(vec![]);
    invalid_config.name = "".to_string();
    let result = invalid_config.validate().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_aggregate_processor_config_validation() {
    // Test valid config
    let config = AggregateProcessorConfig::new(vec![], 60000);
    let result = config.validate().await;
    assert!(result.is_ok());
    
    // Test invalid config (empty name)
    let mut invalid_config = AggregateProcessorConfig::new(vec![], 60000);
    invalid_config.name = "".to_string();
    let result = invalid_config.validate().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_processor_config_as_any() {
    let config = StreamProcessorConfig::new();
    let any_ref = config.as_any();
    let downcast = any_ref.downcast_ref::<StreamProcessorConfig>();
    assert!(downcast.is_some());
}

#[tokio::test]
async fn test_filter_operators() {
    // Test all filter operators
    let operators = vec![
        FilterOperator::Equals,
        FilterOperator::NotEquals,
        FilterOperator::GreaterThan,
        FilterOperator::LessThan,
        FilterOperator::GreaterThanOrEqual,
        FilterOperator::LessThanOrEqual,
        FilterOperator::Contains,
        FilterOperator::NotContains,
        FilterOperator::Regex,
        FilterOperator::In,
        FilterOperator::NotIn,
    ];
    
    for operator in operators {
        let rule = FilterRule {
            name: "test".to_string(),
            field: "test_field".to_string(),
            operator,
            value: "test_value".to_string(),
            enabled: true,
        };
        
        assert_eq!(rule.name, "test");
        assert_eq!(rule.field, "test_field");
        assert_eq!(rule.value, "test_value");
        assert!(rule.enabled);
    }
}

#[tokio::test]
async fn test_transform_rule_types() {
    // Test all transform rule types
    let rule_types = vec![
        TransformRuleType::Copy,
        TransformRuleType::Rename,
        TransformRuleType::Set,
        TransformRuleType::Remove,
        TransformRuleType::Add,
        TransformRuleType::Replace,
    ];
    
    for rule_type in rule_types {
        let rule = TransformRule {
            name: "test".to_string(),
            rule_type,
            source_field: "source".to_string(),
            target_field: "target".to_string(),
            transform_value: Some("value".to_string()),
            enabled: true,
        };
        
        assert_eq!(rule.name, "test");
        assert_eq!(rule.source_field, "source");
        assert_eq!(rule.target_field, "target");
        assert!(rule.enabled);
    }
}

#[tokio::test]
async fn test_aggregation_functions() {
    let function = AggregationFunction {
        name: "sum".to_string(),
        function_type: AggregationFunctionType::Sum,
        source_field: "metric.value".to_string(),
        target_field: "sum_value".to_string(),
    };
    
    assert_eq!(function.name, "sum");
    assert_eq!(function.function_type, AggregationFunctionType::Sum);
    assert_eq!(function.source_field, "metric.value");
    assert_eq!(function.target_field, "sum_value");
}

#[tokio::test]
async fn test_processor_error_handling() {
    // Test that processors handle errors gracefully
    let config = StreamProcessorConfig::new();
    let processor = StreamProcessor::new(&config).await.unwrap();
    
    // Create an empty data stream (should still work)
    let input_stream = create_test_data_stream("empty-stream", vec![]);
    let result = processor.process_stream(input_stream).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_processor_concurrent_access() {
    use std::sync::Arc;
    use tokio::task;
    
    let config = StreamProcessorConfig::new();
    let processor = Arc::new(StreamProcessor::new(&config).await.unwrap());
    
    let mut handles = vec![];
    
    // Spawn multiple tasks to test concurrent access
    for i in 0..5 {
        let processor = processor.clone();
        let handle = task::spawn(async move {
            let input_stream = create_test_data_stream(&format!("stream-{}", i), vec![i as u8]);
            processor.process_stream(input_stream).await
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_processor_memory_usage() {
    let config = StreamProcessorConfig::new();
    let processor = StreamProcessor::new(&config).await.unwrap();
    
    // Process multiple streams to test memory usage
    for i in 0..100 {
        let input_stream = create_test_data_stream(&format!("stream-{}", i), vec![i as u8]);
        let result = processor.process_stream(input_stream).await;
        assert!(result.is_ok());
    }
    
    // Check that stats are updated
    let stats = processor.get_stats().await.unwrap();
    assert!(stats.total_records > 0);
}

#[tokio::test]
async fn test_processor_large_data() {
    let config = StreamProcessorConfig::new();
    let processor = StreamProcessor::new(&config).await.unwrap();
    
    // Create a large data stream
    let large_data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
    let input_stream = create_test_data_stream("large-stream", large_data);
    
    let result = processor.process_stream(input_stream).await;
    assert!(result.is_ok());
    
    let output_stream = result.unwrap();
    assert_eq!(output_stream.data.len(), 10000);
}
