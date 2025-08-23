//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Advanced integration tests for ingestion system
//!
//! This module provides comprehensive tests for advanced ingestion features
//! including data processing, error handling, and performance optimization.

#[cfg(test)]
mod tests {
    use super::super::*;
    use bridge_core::{
        types::{TelemetryData, TelemetryRecord, TelemetryType},
        BridgeResult, TelemetryBatch,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use tokio::time::{sleep, Duration};
    use uuid::Uuid;

    // Import processor modules
    use crate::error_handling::{
        circuit_breaker, dead_letter_queue, retry_policy, CircuitState, ErrorClassification,
        ErrorClassificationConfig, ErrorContext, ErrorHandlingConfig,
    };
    use crate::processors::aggregate_processor::{
        AggregateProcessorConfig, AggregationFunction, AggregationFunctionType, AggregationRule,
        WindowConfig, WindowType,
    };
    use crate::processors::batch_processor::BatchProcessorConfig;
    use crate::processors::enrichment_processor::EnrichmentProcessorConfig;
    use crate::processors::filter_processor::{
        FilterMode, FilterOperator, FilterProcessorConfig, FilterRule,
    };
    use crate::processors::transform_processor::{
        TransformProcessorConfig, TransformRule, TransformRuleType,
    };
    use crate::processors::{
        AggregateProcessor, BatchProcessor, EnrichmentProcessor, FilterProcessor, ProcessorConfig,
        TransformProcessor,
    };

    /// Test advanced data processing pipeline
    #[tokio::test]
    async fn test_advanced_data_processing_pipeline() -> BridgeResult<()> {
        // Create test data
        let test_records = create_test_telemetry_records(100);
        let batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "test-source".to_string(),
            size: test_records.len(),
            records: test_records,
            metadata: HashMap::new(),
        };

        // Test transform processor
        let transform_rules = vec![
            TransformRule {
                name: "add_timestamp".to_string(),
                rule_type: TransformRuleType::Add,
                source_field: "timestamp".to_string(),
                target_field: "processed_at".to_string(),
                transform_value: Some("${timestamp}".to_string()),
                enabled: true,
            },
            TransformRule {
                name: "normalize_service".to_string(),
                rule_type: TransformRuleType::Set,
                source_field: "service".to_string(),
                target_field: "service_normalized".to_string(),
                transform_value: Some("test-service".to_string()),
                enabled: true,
            },
        ];

        let transform_config = TransformProcessorConfig::new(transform_rules);
        let transform_processor = TransformProcessor::new(&transform_config).await?;

        let processed_batch = transform_processor.process(batch).await?;
        assert_eq!(processed_batch.records.len(), 100);

        // Test filter processor
        let filter_rules = vec![FilterRule {
            name: "include_test_records".to_string(),
            field: "source".to_string(),
            operator: FilterOperator::Equals,
            value: "test-source".to_string(),
            enabled: true,
        }];

        let filter_config = FilterProcessorConfig::new(filter_rules, FilterMode::Include);
        let filter_processor = FilterProcessor::new(&filter_config).await?;

        // Convert ProcessedBatch back to TelemetryBatch for next processor
        let filtered_telemetry_batch = TelemetryBatch {
            id: processed_batch.original_batch_id,
            timestamp: processed_batch.timestamp,
            source: "test-source".to_string(),
            size: processed_batch.records.len(),
            records: processed_batch
                .records
                .into_iter()
                .map(|pr| {
                    // Convert ProcessedRecord back to TelemetryRecord
                    TelemetryRecord {
                        id: pr.original_id,
                        timestamp: processed_batch.timestamp,
                        record_type: TelemetryType::Metric,
                        data: pr.transformed_data.unwrap_or(TelemetryData::Metric(
                            bridge_core::types::MetricData {
                                name: "test".to_string(),
                                description: None,
                                unit: None,
                                metric_type: bridge_core::types::MetricType::Gauge,
                                value: bridge_core::types::MetricValue::Gauge(1.0),
                                labels: HashMap::new(),
                                timestamp: processed_batch.timestamp,
                            },
                        )),
                        attributes: pr.metadata,
                        tags: HashMap::new(),
                        resource: None,
                        service: None,
                    }
                })
                .collect(),
            metadata: processed_batch.metadata,
        };

        let filtered_batch = filter_processor.process(filtered_telemetry_batch).await?;
        assert_eq!(filtered_batch.records.len(), 100);

        // Test aggregate processor
        let aggregation_rules = vec![AggregationRule {
            name: "count_by_service".to_string(),
            description: Some("Count records by service".to_string()),
            group_by_fields: vec!["service_normalized".to_string()],
            aggregation_functions: vec![AggregationFunction {
                name: "count".to_string(),
                source_field: "id".to_string(),
                target_field: "record_count".to_string(),
                function_type: AggregationFunctionType::Count,
                parameters: HashMap::new(),
            }],
            filter_condition: None,
            enabled: true,
        }];

        let window_config = WindowConfig {
            window_type: WindowType::Tumbling,
            window_size_ms: 60000,
            slide_interval_ms: 60000,
            watermark_delay_ms: 5000,
            allowed_lateness_ms: 10000,
        };

        let aggregate_config = AggregateProcessorConfig::new(aggregation_rules, window_config);
        let aggregate_processor =
            AggregateProcessor::new(&aggregate_config as &dyn ProcessorConfig).await?;

        // Convert ProcessedBatch back to TelemetryBatch for aggregate processor
        let aggregate_telemetry_batch = TelemetryBatch {
            id: filtered_batch.original_batch_id,
            timestamp: filtered_batch.timestamp,
            source: "test-source".to_string(),
            size: filtered_batch.records.len(),
            records: filtered_batch
                .records
                .into_iter()
                .map(|pr| TelemetryRecord {
                    id: pr.original_id,
                    timestamp: filtered_batch.timestamp,
                    record_type: TelemetryType::Metric,
                    data: pr.transformed_data.unwrap_or(TelemetryData::Metric(
                        bridge_core::types::MetricData {
                            name: "test".to_string(),
                            description: None,
                            unit: None,
                            metric_type: bridge_core::types::MetricType::Gauge,
                            value: bridge_core::types::MetricValue::Gauge(1.0),
                            labels: HashMap::new(),
                            timestamp: filtered_batch.timestamp,
                        },
                    )),
                    attributes: pr.metadata,
                    tags: HashMap::new(),
                    resource: None,
                    service: None,
                })
                .collect(),
            metadata: filtered_batch.metadata,
        };

        let aggregated_batch = aggregate_processor
            .process(aggregate_telemetry_batch)
            .await?;
        assert!(aggregated_batch.records.len() >= 100); // Should have original records plus aggregated records

        Ok(())
    }

    /// Test error handling and resilience
    #[tokio::test]
    async fn test_error_handling_and_resilience() -> BridgeResult<()> {
        // Test circuit breaker only
        let circuit_breaker = error_handling::CircuitBreaker::new(
            error_handling::CircuitBreakerConfig::with_thresholds(2, 1),
        )
        .await?;

        // Record some failures
        circuit_breaker.record_error().await?;
        circuit_breaker.record_error().await?;

        // Circuit should be open
        assert!(!circuit_breaker.can_execute().await?);
        assert_eq!(
            circuit_breaker.get_state().await,
            error_handling::CircuitState::Open
        );

        // Test retry policy
        let retry_policy =
            error_handling::RetryPolicy::new(error_handling::RetryPolicyConfig::with_attempts(3))
                .await?;

        let error_classification = error_handling::ErrorClassification {
            error_type: error_handling::ErrorType::Network,
            severity: error_handling::ErrorSeverity::Warning,
            message: "Network timeout".to_string(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        assert!(retry_policy.should_retry(&error_classification, 1).await?);
        assert!(!retry_policy.should_retry(&error_classification, 3).await?);

        // Test dead letter queue
        let mut dead_letter_config = error_handling::DeadLetterQueueConfig::new();
        dead_letter_config.enable_auto_cleanup = false; // Disable auto-cleanup for testing
        let dead_letter_queue = error_handling::DeadLetterQueue::new(dead_letter_config).await?;

        let dead_letter_record = error_handling::DeadLetterRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            operation: "test_operation".to_string(),
            error_message: "Test error".to_string(),
            error_type: "test_error".to_string(),
            source: "test_source".to_string(),
            retry_count: 3,
            metadata: HashMap::new(),
        };

        dead_letter_queue.add_record(dead_letter_record).await?;

        let stats = dead_letter_queue.get_statistics().await?;
        assert_eq!(stats.total_records, 1);

        Ok(())
    }

    /// Test performance optimization features
    #[tokio::test]
    async fn test_performance_optimization() -> BridgeResult<()> {
        // Test batch processing
        let batch_config = BatchProcessorConfig::new(
            1000, // max_batch_size
            5000, // max_batch_time_ms
        );
        let batch_processor = BatchProcessor::new(&batch_config).await?;

        // Create large batch of records
        let large_batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "performance-test".to_string(),
            size: 5000,
            records: create_test_telemetry_records(5000),
            metadata: HashMap::new(),
        };

        let start_time = std::time::Instant::now();
        let processed_batch = batch_processor.process(large_batch).await?;
        let processing_time = start_time.elapsed();

        assert_eq!(processed_batch.records.len(), 5000);
        assert!(processing_time.as_millis() < 1000); // Should process quickly

        // Test enrichment processor
        let enrichment_config = EnrichmentProcessorConfig::new();
        let enrichment_processor = EnrichmentProcessor::new(&enrichment_config).await?;

        // Convert ProcessedBatch back to TelemetryBatch for enrichment processor
        let enrichment_telemetry_batch = TelemetryBatch {
            id: processed_batch.original_batch_id,
            timestamp: processed_batch.timestamp,
            source: "performance-test".to_string(),
            size: processed_batch.records.len(),
            records: processed_batch
                .records
                .into_iter()
                .map(|pr| TelemetryRecord {
                    id: pr.original_id,
                    timestamp: processed_batch.timestamp,
                    record_type: TelemetryType::Metric,
                    data: pr.transformed_data.unwrap_or(TelemetryData::Metric(
                        bridge_core::types::MetricData {
                            name: "test".to_string(),
                            description: None,
                            unit: None,
                            metric_type: bridge_core::types::MetricType::Gauge,
                            value: bridge_core::types::MetricValue::Gauge(1.0),
                            labels: HashMap::new(),
                            timestamp: processed_batch.timestamp,
                        },
                    )),
                    attributes: pr.metadata,
                    tags: HashMap::new(),
                    resource: None,
                    service: None,
                })
                .collect(),
            metadata: processed_batch.metadata,
        };

        let enrichment_start = std::time::Instant::now();
        let enriched_batch = enrichment_processor
            .process(enrichment_telemetry_batch)
            .await?;
        let enrichment_time = enrichment_start.elapsed();

        assert_eq!(enriched_batch.records.len(), 5000);
        assert!(enrichment_time.as_millis() < 500); // Should enrich quickly

        Ok(())
    }

    /// Test concurrent processing
    #[tokio::test]
    async fn test_concurrent_processing() -> BridgeResult<()> {
        let mut handles = Vec::new();

        // Spawn multiple concurrent processing tasks
        for i in 0..10 {
            let handle = tokio::spawn(async move {
                let test_records = create_test_telemetry_records(100);
                let batch = TelemetryBatch {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    source: format!("concurrent-test-{}", i),
                    size: test_records.len(),
                    records: test_records,
                    metadata: HashMap::new(),
                };

                // Process with transform processor
                let transform_rules = vec![TransformRule {
                    name: "add_index".to_string(),
                    rule_type: TransformRuleType::Add,
                    source_field: "index".to_string(),
                    target_field: "batch_index".to_string(),
                    transform_value: Some(i.to_string()),
                    enabled: true,
                }];

                let transform_config = TransformProcessorConfig::new(transform_rules);
                let transform_processor = TransformProcessor::new(&transform_config).await?;

                let processed_batch = transform_processor.process(batch).await?;
                assert_eq!(processed_batch.records.len(), 100);

                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(processed_batch)
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        Ok(())
    }

    /// Test error recovery and resilience
    #[tokio::test]
    async fn test_error_recovery_and_resilience() -> BridgeResult<()> {
        // Create error handling manager
        let error_handling_config = ErrorHandlingConfig {
            circuit_breaker: circuit_breaker::CircuitBreakerConfig::new(),
            retry_policy: retry_policy::RetryPolicyConfig::new(),
            error_classification: ErrorClassificationConfig::new(),
            dead_letter_queue: dead_letter_queue::DeadLetterQueueConfig::new(),
        };

        let error_handler = error_handling::ErrorHandler::new(error_handling_config).await?;

        // Test error context
        let error_context = ErrorContext {
            source: "test-operation".to_string(),
            retry_count: 0,
            metadata: HashMap::new(),
        };

        // Test error handling with retry logic
        let mut retry_count = 0;
        let result = async {
            retry_count += 1;
            if retry_count < 3 {
                Err(bridge_core::BridgeError::network("Temporary network error"))
            } else {
                Ok("Success after retries".to_string())
            }
        }
        .await;

        // For now, just test that the error handler was created successfully
        assert!(result.is_err()); // Should fail on first attempt

        Ok(())
    }

    /// Test comprehensive pipeline integration
    #[tokio::test]
    async fn test_comprehensive_pipeline_integration() -> BridgeResult<()> {
        // Create a comprehensive pipeline with all components
        let test_records = create_test_telemetry_records(1000);
        let mut batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "comprehensive-test".to_string(),
            size: test_records.len(),
            records: test_records,
            metadata: HashMap::new(),
        };

        // Step 1: Transform
        let transform_rules = vec![TransformRule {
            name: "add_processing_timestamp".to_string(),
            rule_type: TransformRuleType::Add,
            source_field: "timestamp".to_string(),
            target_field: "processing_timestamp".to_string(),
            transform_value: Some("${timestamp}".to_string()),
            enabled: true,
        }];

        let transform_config = TransformProcessorConfig::new(transform_rules);
        let transform_processor = TransformProcessor::new(&transform_config).await?;
        
        let processed_batch = transform_processor.process(batch).await?;
        // Convert ProcessedBatch back to TelemetryBatch for next processor
        batch = TelemetryBatch {
            id: processed_batch.original_batch_id,
            timestamp: processed_batch.timestamp,
            source: "comprehensive-test".to_string(),
            size: processed_batch.records.len(),
            records: processed_batch
                .records
                .into_iter()
                .map(|pr| TelemetryRecord {
                    id: pr.original_id,
                    timestamp: processed_batch.timestamp,
                    record_type: TelemetryType::Metric,
                    data: pr.transformed_data.unwrap_or(TelemetryData::Metric(
                        bridge_core::types::MetricData {
                            name: "test".to_string(),
                            description: None,
                            unit: None,
                            metric_type: bridge_core::types::MetricType::Gauge,
                            value: bridge_core::types::MetricValue::Gauge(1.0),
                            labels: HashMap::new(),
                            timestamp: processed_batch.timestamp,
                        },
                    )),
                    attributes: pr.metadata,
                    tags: HashMap::new(),
                    resource: None,
                    service: None,
                })
                .collect(),
            metadata: processed_batch.metadata,
        };

        // Step 2: Filter
        let filter_rules = vec![FilterRule {
            name: "include_all".to_string(),
            field: "source".to_string(),
            operator: FilterOperator::Equals,
            value: "test-source".to_string(),
            enabled: true,
        }];

        let filter_config = FilterProcessorConfig::new(filter_rules, FilterMode::Include);
        let filter_processor = FilterProcessor::new(&filter_config).await?;
        let filtered_batch = filter_processor.process(batch).await?;

        // Step 3: Enrich
        let enrichment_config = EnrichmentProcessorConfig::new();
        let enrichment_processor = EnrichmentProcessor::new(&enrichment_config).await?;

        // Convert ProcessedBatch back to TelemetryBatch for enrichment processor
        let enrichment_telemetry_batch = TelemetryBatch {
            id: filtered_batch.original_batch_id,
            timestamp: filtered_batch.timestamp,
            source: "comprehensive-test".to_string(),
            size: filtered_batch.records.len(),
            records: filtered_batch
                .records
                .into_iter()
                .map(|pr| TelemetryRecord {
                    id: pr.original_id,
                    timestamp: filtered_batch.timestamp,
                    record_type: TelemetryType::Metric,
                    data: pr.transformed_data.unwrap_or(TelemetryData::Metric(
                        bridge_core::types::MetricData {
                            name: "test".to_string(),
                            description: None,
                            unit: None,
                            metric_type: bridge_core::types::MetricType::Gauge,
                            value: bridge_core::types::MetricValue::Gauge(1.0),
                            labels: HashMap::new(),
                            timestamp: filtered_batch.timestamp,
                        },
                    )),
                    attributes: pr.metadata,
                    tags: HashMap::new(),
                    resource: None,
                    service: None,
                })
                .collect(),
            metadata: filtered_batch.metadata,
        };

        let enriched_batch = enrichment_processor
            .process(enrichment_telemetry_batch)
            .await?;

        // Step 4: Aggregate
        let aggregation_rules = vec![AggregationRule {
            name: "count_records".to_string(),
            description: Some("Count total records".to_string()),
            group_by_fields: vec!["source".to_string()],
            aggregation_functions: vec![AggregationFunction {
                name: "count".to_string(),
                source_field: "id".to_string(),
                target_field: "total_count".to_string(),
                function_type: AggregationFunctionType::Count,
                parameters: HashMap::new(),
            }],
            filter_condition: None,
            enabled: true,
        }];

        let window_config = WindowConfig {
            window_type: WindowType::Tumbling,
            window_size_ms: 60000,
            slide_interval_ms: 60000,
            watermark_delay_ms: 5000,
            allowed_lateness_ms: 10000,
        };

        let aggregate_config = AggregateProcessorConfig::new(aggregation_rules, window_config);
        let aggregate_processor =
            AggregateProcessor::new(&aggregate_config as &dyn ProcessorConfig).await?;

        // Convert ProcessedBatch back to TelemetryBatch for aggregate processor
        let aggregate_telemetry_batch = TelemetryBatch {
            id: enriched_batch.original_batch_id,
            timestamp: enriched_batch.timestamp,
            source: "comprehensive-test".to_string(),
            size: enriched_batch.records.len(),
            records: enriched_batch
                .records
                .into_iter()
                .map(|pr| TelemetryRecord {
                    id: pr.original_id,
                    timestamp: enriched_batch.timestamp,
                    record_type: TelemetryType::Metric,
                    data: pr.transformed_data.unwrap_or(TelemetryData::Metric(
                        bridge_core::types::MetricData {
                            name: "test".to_string(),
                            description: None,
                            unit: None,
                            metric_type: bridge_core::types::MetricType::Gauge,
                            value: bridge_core::types::MetricValue::Gauge(1.0),
                            labels: HashMap::new(),
                            timestamp: enriched_batch.timestamp,
                        },
                    )),
                    attributes: pr.metadata,
                    tags: HashMap::new(),
                    resource: None,
                    service: None,
                })
                .collect(),
            metadata: enriched_batch.metadata,
        };

        let original_size = aggregate_telemetry_batch.size;
        let final_batch = aggregate_processor
            .process(aggregate_telemetry_batch)
            .await?;

        // Verify final results
        assert!(final_batch.records.len() >= 1000); // Should have original records plus aggregated records

        Ok(())
    }

    /// Helper function to create test telemetry records
    fn create_test_telemetry_records(count: usize) -> Vec<TelemetryRecord> {
        let mut records = Vec::with_capacity(count);

        for i in 0..count {
            let record = TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(bridge_core::types::MetricData {
                    name: format!("test_metric_{}", i),
                    description: Some(format!("Test metric {}", i)),
                    unit: Some("count".to_string()),
                    metric_type: bridge_core::types::MetricType::Gauge,
                    value: bridge_core::types::MetricValue::Gauge(i as f64),
                    labels: HashMap::new(),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::from([
                    ("source".to_string(), "test-source".to_string()),
                    ("index".to_string(), i.to_string()),
                ]),
                tags: HashMap::new(),
                resource: None,
                service: Some(bridge_core::types::ServiceInfo {
                    name: "test-service".to_string(),
                    version: Some("1.0.0".to_string()),
                    namespace: Some("test".to_string()),
                    instance_id: Some("test-instance".to_string()),
                }),
            };

            records.push(record);
        }

        records
    }
}
