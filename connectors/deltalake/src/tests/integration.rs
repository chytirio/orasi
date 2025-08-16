//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Delta Lake connector integration tests
//! 
//! This module contains integration tests that test the complete
//! workflow of the Delta Lake connector.

use crate::config::DeltaLakeConfig;
use crate::connector::DeltaLakeConnector;
use crate::writer::DeltaLakeWriter;
use crate::reader::DeltaLakeReader;
use crate::error::DeltaLakeResult;
use crate::tests::utils;
use bridge_core::types::{
    MetricsBatch, MetricsRecord, TracesBatch, TracesRecord, LogsBatch, LogsRecord,
    MetricsQuery, TracesQuery, LogsQuery, TelemetryBatch
};

#[cfg(test)]
mod end_to_end_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_telemetry_workflow() {
        // Setup
        let connector = utils::create_test_connector().await.unwrap();
        let writer = connector.writer().await.unwrap();
        let reader = connector.reader().await.unwrap();

        // Write metrics
        let metrics_batch = MetricsBatch {
            records: vec![
                MetricsRecord {
                    name: "cpu_usage".to_string(),
                    value: 75.5,
                    timestamp: chrono::Utc::now(),
                    labels: {
                        let mut labels = std::collections::HashMap::new();
                        labels.insert("service".to_string(), "web-server".to_string());
                        labels.insert("instance".to_string(), "web-1".to_string());
                        labels
                    },
                },
                MetricsRecord {
                    name: "memory_usage".to_string(),
                    value: 85.2,
                    timestamp: chrono::Utc::now(),
                    labels: {
                        let mut labels = std::collections::HashMap::new();
                        labels.insert("service".to_string(), "web-server".to_string());
                        labels.insert("instance".to_string(), "web-1".to_string());
                        labels
                    },
                },
            ],
        };

        let write_result = writer.write_metrics(metrics_batch).await;
        assert!(write_result.is_ok());

        // Write traces
        let traces_batch = TracesBatch {
            records: vec![
                TracesRecord {
                    trace_id: "trace-123".to_string(),
                    span_id: "span-456".to_string(),
                    parent_span_id: None,
                    name: "http_request".to_string(),
                    service_name: "web-server".to_string(),
                    start_time: chrono::Utc::now(),
                    end_time: Some(chrono::Utc::now() + chrono::Duration::milliseconds(150)),
                    attributes: {
                        let mut attrs = std::collections::HashMap::new();
                        attrs.insert("http.method".to_string(), "GET".to_string());
                        attrs.insert("http.url".to_string(), "/api/users".to_string());
                        attrs
                    },
                },
            ],
        };

        let write_result = writer.write_traces(traces_batch).await;
        assert!(write_result.is_ok());

        // Write logs
        let logs_batch = LogsBatch {
            records: vec![
                LogsRecord {
                    timestamp: chrono::Utc::now(),
                    level: "INFO".to_string(),
                    message: "User request processed successfully".to_string(),
                    service_name: "web-server".to_string(),
                    attributes: {
                        let mut attrs = std::collections::HashMap::new();
                        attrs.insert("user_id".to_string(), "12345".to_string());
                        attrs.insert("request_id".to_string(), "req-789".to_string());
                        attrs
                    },
                },
            ],
        };

        let write_result = writer.write_logs(logs_batch).await;
        assert!(write_result.is_ok());

        // Flush to ensure data is written
        let flush_result = writer.flush().await;
        assert!(flush_result.is_ok());

        // Query metrics
        let metrics_query = MetricsQuery {
            name: Some("cpu_usage".to_string()),
            start_time: Some(chrono::Utc::now() - chrono::Duration::minutes(5)),
            end_time: Some(chrono::Utc::now() + chrono::Duration::minutes(5)),
            labels: {
                let mut labels = std::collections::HashMap::new();
                labels.insert("service".to_string(), "web-server".to_string());
                labels
            },
            limit: Some(100),
        };

        let metrics_result = reader.query_metrics(metrics_query).await;
        assert!(metrics_result.is_ok());

        // Query traces
        let traces_query = TracesQuery {
            trace_id: Some("trace-123".to_string()),
            start_time: Some(chrono::Utc::now() - chrono::Duration::minutes(5)),
            end_time: Some(chrono::Utc::now() + chrono::Duration::minutes(5)),
            service_name: Some("web-server".to_string()),
            limit: Some(100),
        };

        let traces_result = reader.query_traces(traces_query).await;
        assert!(traces_result.is_ok());

        // Query logs
        let logs_query = LogsQuery {
            start_time: Some(chrono::Utc::now() - chrono::Duration::minutes(5)),
            end_time: Some(chrono::Utc::now() + chrono::Duration::minutes(5)),
            service_name: Some("web-server".to_string()),
            level: Some("INFO".to_string()),
            limit: Some(100),
        };

        let logs_result = reader.query_logs(logs_query).await;
        assert!(logs_result.is_ok());

        // Cleanup
        let shutdown_result = connector.shutdown().await;
        assert!(shutdown_result.is_ok());
    }

    #[tokio::test]
    async fn test_batch_operations() {
        let connector = utils::create_test_connector().await.unwrap();
        let writer = connector.writer().await.unwrap();

        // Create a mixed telemetry batch
        let telemetry_batch = TelemetryBatch {
            metrics: Some(MetricsBatch {
                records: vec![
                    MetricsRecord {
                        name: "batch_test_metric".to_string(),
                        value: 42.0,
                        timestamp: chrono::Utc::now(),
                        labels: std::collections::HashMap::new(),
                    },
                ],
            }),
            traces: Some(TracesBatch {
                records: vec![
                    TracesRecord {
                        trace_id: "batch-trace-123".to_string(),
                        span_id: "batch-span-456".to_string(),
                        parent_span_id: None,
                        name: "batch_test_span".to_string(),
                        service_name: "test-service".to_string(),
                        start_time: chrono::Utc::now(),
                        end_time: Some(chrono::Utc::now() + chrono::Duration::milliseconds(100)),
                        attributes: std::collections::HashMap::new(),
                    },
                ],
            }),
            logs: Some(LogsBatch {
                records: vec![
                    LogsRecord {
                        timestamp: chrono::Utc::now(),
                        level: "DEBUG".to_string(),
                        message: "Batch test log message".to_string(),
                        service_name: "test-service".to_string(),
                        attributes: std::collections::HashMap::new(),
                    },
                ],
            }),
        };

        let write_result = writer.write_batch(telemetry_batch).await;
        assert!(write_result.is_ok());

        let flush_result = writer.flush().await;
        assert!(flush_result.is_ok());

        let shutdown_result = connector.shutdown().await;
        assert!(shutdown_result.is_ok());
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let connector = utils::create_test_connector().await.unwrap();
        let writer = connector.writer().await.unwrap();
        let reader = connector.reader().await.unwrap();

        // Spawn multiple concurrent write operations
        let write_handles: Vec<_> = (0..5).map(|i| {
            let writer = writer.clone();
            tokio::spawn(async move {
                let batch = MetricsBatch {
                    records: vec![
                        MetricsRecord {
                            name: format!("concurrent_metric_{}", i),
                            value: i as f64,
                            timestamp: chrono::Utc::now(),
                            labels: std::collections::HashMap::new(),
                        },
                    ],
                };
                writer.write_metrics(batch).await
            })
        }).collect();

        // Wait for all writes to complete
        for handle in write_handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        // Flush and verify
        let flush_result = writer.flush().await;
        assert!(flush_result.is_ok());

        // Query the written data
        let query = MetricsQuery {
            name: Some("concurrent_metric_0".to_string()),
            start_time: Some(chrono::Utc::now() - chrono::Duration::minutes(1)),
            end_time: Some(chrono::Utc::now() + chrono::Duration::minutes(1)),
            labels: std::collections::HashMap::new(),
            limit: Some(100),
        };

        let query_result = reader.query_metrics(query).await;
        assert!(query_result.is_ok());

        let shutdown_result = connector.shutdown().await;
        assert!(shutdown_result.is_ok());
    }

    #[tokio::test]
    async fn test_error_handling_and_recovery() {
        // Test with invalid configuration
        let invalid_config = DeltaLakeConfig::new(
            "".to_string(), // Invalid empty path
            "test_table".to_string(),
        );

        let connector_result = DeltaLakeConnector::connect(invalid_config).await;
        assert!(connector_result.is_err());

        // Test with valid configuration
        let connector = utils::create_test_connector().await.unwrap();
        let writer = connector.writer().await.unwrap();

        // Test health check
        let health_result = connector.health_check().await;
        assert!(health_result.is_ok());

        // Test stats
        let stats_result = connector.get_stats().await;
        assert!(stats_result.is_ok());

        let shutdown_result = connector.shutdown().await;
        assert!(shutdown_result.is_ok());
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let connector = utils::create_test_connector().await.unwrap();
        let writer = connector.writer().await.unwrap();

        // Write multiple batches to test performance
        let start_time = std::time::Instant::now();
        
        for i in 0..100 {
            let batch = MetricsBatch {
                records: vec![
                    MetricsRecord {
                        name: format!("perf_metric_{}", i),
                        value: i as f64,
                        timestamp: chrono::Utc::now(),
                        labels: std::collections::HashMap::new(),
                    },
                ],
            };
            
            let result = writer.write_metrics(batch).await;
            assert!(result.is_ok());
        }

        let flush_result = writer.flush().await;
        assert!(flush_result.is_ok());

        let elapsed = start_time.elapsed();
        println!("Wrote 100 metric batches in {:?}", elapsed);

        // Verify performance is reasonable (should complete in under 5 seconds)
        assert!(elapsed < std::time::Duration::from_secs(5));

        let shutdown_result = connector.shutdown().await;
        assert!(shutdown_result.is_ok());
    }
}

#[cfg(test)]
mod data_consistency_tests {
    use super::*;

    #[tokio::test]
    async fn test_data_integrity() {
        let connector = utils::create_test_connector().await.unwrap();
        let writer = connector.writer().await.unwrap();
        let reader = connector.reader().await.unwrap();

        // Write specific test data
        let test_metric_name = "integrity_test_metric";
        let test_value = 123.456;
        let test_timestamp = chrono::Utc::now();

        let batch = MetricsBatch {
            records: vec![
                MetricsRecord {
                    name: test_metric_name.to_string(),
                    value: test_value,
                    timestamp: test_timestamp,
                    labels: std::collections::HashMap::new(),
                },
            ],
        };

        let write_result = writer.write_metrics(batch).await;
        assert!(write_result.is_ok());

        let flush_result = writer.flush().await;
        assert!(flush_result.is_ok());

        // Query and verify the data
        let query = MetricsQuery {
            name: Some(test_metric_name.to_string()),
            start_time: Some(test_timestamp - chrono::Duration::seconds(1)),
            end_time: Some(test_timestamp + chrono::Duration::seconds(1)),
            labels: std::collections::HashMap::new(),
            limit: Some(100),
        };

        let query_result = reader.query_metrics(query).await;
        assert!(query_result.is_ok());

        let metrics_result = query_result.unwrap();
        assert_eq!(metrics_result.records.len(), 1);
        assert_eq!(metrics_result.records[0].name, test_metric_name);
        assert_eq!(metrics_result.records[0].value, test_value);

        let shutdown_result = connector.shutdown().await;
        assert!(shutdown_result.is_ok());
    }

    #[tokio::test]
    async fn test_schema_evolution_integration() {
        let connector = utils::create_test_connector().await.unwrap();
        let writer = connector.writer().await.unwrap();

        // Write data with initial schema
        let batch1 = MetricsBatch {
            records: vec![
                MetricsRecord {
                    name: "schema_test_metric".to_string(),
                    value: 1.0,
                    timestamp: chrono::Utc::now(),
                    labels: std::collections::HashMap::new(),
                },
            ],
        };

        let write_result1 = writer.write_metrics(batch1).await;
        assert!(write_result1.is_ok());

        // Write data with additional labels (schema evolution)
        let batch2 = MetricsBatch {
            records: vec![
                MetricsRecord {
                    name: "schema_test_metric".to_string(),
                    value: 2.0,
                    timestamp: chrono::Utc::now(),
                    labels: {
                        let mut labels = std::collections::HashMap::new();
                        labels.insert("new_field".to_string(), "new_value".to_string());
                        labels
                    },
                },
            ],
        };

        let write_result2 = writer.write_metrics(batch2).await;
        assert!(write_result2.is_ok());

        let flush_result = writer.flush().await;
        assert!(flush_result.is_ok());

        let shutdown_result = connector.shutdown().await;
        assert!(shutdown_result.is_ok());
    }
}
