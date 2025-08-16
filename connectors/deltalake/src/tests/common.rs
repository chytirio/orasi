//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Common test utilities and fixtures
//! 
//! This module provides shared test utilities, fixtures, and helpers
//! that can be used across all connector tests.

use bridge_core::types::{
    MetricsBatch, MetricsRecord, TracesBatch, TracesRecord, LogsBatch, LogsRecord,
    MetricsQuery, TracesQuery, LogsQuery, TelemetryBatch
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Test data generators
pub mod generators {
    use super::*;

    /// Generate a test metrics record
    pub fn generate_metrics_record(
        name: &str,
        value: f64,
        timestamp: DateTime<Utc>,
        labels: HashMap<String, String>,
    ) -> MetricsRecord {
        MetricsRecord {
            name: name.to_string(),
            value,
            timestamp,
            labels,
        }
    }

    /// Generate a test metrics batch
    pub fn generate_metrics_batch(
        count: usize,
        base_name: &str,
        start_time: DateTime<Utc>,
    ) -> MetricsBatch {
        let records: Vec<MetricsRecord> = (0..count)
            .map(|i| {
                generate_metrics_record(
                    &format!("{}_{}", base_name, i),
                    i as f64,
                    start_time + chrono::Duration::seconds(i as i64),
                    {
                        let mut labels = HashMap::new();
                        labels.insert("service".to_string(), "test-service".to_string());
                        labels.insert("instance".to_string(), format!("instance-{}", i));
                        labels
                    },
                )
            })
            .collect();

        MetricsBatch { records }
    }

    /// Generate a test traces record
    pub fn generate_traces_record(
        trace_id: &str,
        span_id: &str,
        name: &str,
        service_name: &str,
        start_time: DateTime<Utc>,
        duration_ms: u64,
        attributes: HashMap<String, String>,
    ) -> TracesRecord {
        TracesRecord {
            trace_id: trace_id.to_string(),
            span_id: span_id.to_string(),
            parent_span_id: None,
            name: name.to_string(),
            service_name: service_name.to_string(),
            start_time,
            end_time: Some(start_time + chrono::Duration::milliseconds(duration_ms as i64)),
            attributes,
        }
    }

    /// Generate a test traces batch
    pub fn generate_traces_batch(
        count: usize,
        base_trace_id: &str,
        service_name: &str,
        start_time: DateTime<Utc>,
    ) -> TracesBatch {
        let records: Vec<TracesRecord> = (0..count)
            .map(|i| {
                generate_traces_record(
                    &format!("{}-{}", base_trace_id, i),
                    &format!("span-{}", i),
                    &format!("test-span-{}", i),
                    service_name,
                    start_time + chrono::Duration::seconds(i as i64),
                    100 + (i as u64 * 10),
                    {
                        let mut attrs = HashMap::new();
                        attrs.insert("http.method".to_string(), "GET".to_string());
                        attrs.insert("http.url".to_string(), format!("/api/test/{}", i));
                        attrs
                    },
                )
            })
            .collect();

        TracesBatch { records }
    }

    /// Generate a test logs record
    pub fn generate_logs_record(
        level: &str,
        message: &str,
        service_name: &str,
        timestamp: DateTime<Utc>,
        attributes: HashMap<String, String>,
    ) -> LogsRecord {
        LogsRecord {
            timestamp,
            level: level.to_string(),
            message: message.to_string(),
            service_name: service_name.to_string(),
            attributes,
        }
    }

    /// Generate a test logs batch
    pub fn generate_logs_batch(
        count: usize,
        service_name: &str,
        start_time: DateTime<Utc>,
    ) -> LogsBatch {
        let levels = vec!["DEBUG", "INFO", "WARN", "ERROR"];
        let records: Vec<LogsRecord> = (0..count)
            .map(|i| {
                let level = levels[i % levels.len()];
                generate_logs_record(
                    level,
                    &format!("Test log message {}", i),
                    service_name,
                    start_time + chrono::Duration::seconds(i as i64),
                    {
                        let mut attrs = HashMap::new();
                        attrs.insert("request_id".to_string(), format!("req-{}", i));
                        attrs.insert("user_id".to_string(), format!("user-{}", i));
                        attrs
                    },
                )
            })
            .collect();

        LogsBatch { records }
    }

    /// Generate a mixed telemetry batch
    pub fn generate_telemetry_batch(
        metrics_count: usize,
        traces_count: usize,
        logs_count: usize,
        start_time: DateTime<Utc>,
    ) -> TelemetryBatch {
        TelemetryBatch {
            metrics: Some(generate_metrics_batch(metrics_count, "mixed_metric", start_time)),
            traces: Some(generate_traces_batch(traces_count, "mixed-trace", "mixed-service", start_time)),
            logs: Some(generate_logs_batch(logs_count, "mixed-service", start_time)),
        }
    }
}

/// Test query generators
pub mod queries {
    use super::*;

    /// Generate a metrics query
    pub fn generate_metrics_query(
        name: Option<String>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        labels: HashMap<String, String>,
        limit: Option<usize>,
    ) -> MetricsQuery {
        MetricsQuery {
            name,
            start_time: Some(start_time),
            end_time: Some(end_time),
            labels,
            limit,
        }
    }

    /// Generate a traces query
    pub fn generate_traces_query(
        trace_id: Option<String>,
        service_name: Option<String>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<usize>,
    ) -> TracesQuery {
        TracesQuery {
            trace_id,
            start_time: Some(start_time),
            end_time: Some(end_time),
            service_name,
            limit,
        }
    }

    /// Generate a logs query
    pub fn generate_logs_query(
        service_name: Option<String>,
        level: Option<String>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<usize>,
    ) -> LogsQuery {
        LogsQuery {
            start_time: Some(start_time),
            end_time: Some(end_time),
            service_name,
            level,
            limit,
        }
    }
}

/// Test assertions and validators
pub mod assertions {
    use super::*;

    /// Assert that a metrics batch has the expected structure
    pub fn assert_metrics_batch_structure(batch: &MetricsBatch) {
        assert!(!batch.records.is_empty(), "Metrics batch should not be empty");
        
        for record in &batch.records {
            assert!(!record.name.is_empty(), "Metric name should not be empty");
            assert!(record.value.is_finite(), "Metric value should be finite");
            assert!(record.timestamp <= Utc::now(), "Timestamp should not be in the future");
        }
    }

    /// Assert that a traces batch has the expected structure
    pub fn assert_traces_batch_structure(batch: &TracesBatch) {
        assert!(!batch.records.is_empty(), "Traces batch should not be empty");
        
        for record in &batch.records {
            assert!(!record.trace_id.is_empty(), "Trace ID should not be empty");
            assert!(!record.span_id.is_empty(), "Span ID should not be empty");
            assert!(!record.name.is_empty(), "Span name should not be empty");
            assert!(!record.service_name.is_empty(), "Service name should not be empty");
            assert!(record.start_time <= Utc::now(), "Start time should not be in the future");
            
            if let Some(end_time) = record.end_time {
                assert!(end_time >= record.start_time, "End time should be after start time");
            }
        }
    }

    /// Assert that a logs batch has the expected structure
    pub fn assert_logs_batch_structure(batch: &LogsBatch) {
        assert!(!batch.records.is_empty(), "Logs batch should not be empty");
        
        for record in &batch.records {
            assert!(!record.level.is_empty(), "Log level should not be empty");
            assert!(!record.message.is_empty(), "Log message should not be empty");
            assert!(!record.service_name.is_empty(), "Service name should not be empty");
            assert!(record.timestamp <= Utc::now(), "Timestamp should not be in the future");
        }
    }

    /// Assert that a telemetry batch has the expected structure
    pub fn assert_telemetry_batch_structure(batch: &TelemetryBatch) {
        if let Some(ref metrics) = batch.metrics {
            assert_metrics_batch_structure(metrics);
        }
        
        if let Some(ref traces) = batch.traces {
            assert_traces_batch_structure(traces);
        }
        
        if let Some(ref logs) = batch.logs {
            assert_logs_batch_structure(logs);
        }
    }

    /// Assert that a write result is successful
    pub fn assert_write_result_success(result: &bridge_core::types::WriteResult) {
        assert!(result.records_written > 0, "Should have written at least one record");
        assert!(result.bytes_written >= 0, "Bytes written should be non-negative");
        assert!(result.write_time_ms >= 0, "Write time should be non-negative");
    }

    /// Assert that a query result has the expected structure
    pub fn assert_query_result_structure<T>(result: &bridge_core::types::QueryResult<T>) {
        assert!(result.total_count >= 0, "Total count should be non-negative");
        assert!(result.query_time_ms >= 0, "Query time should be non-negative");
    }
}

/// Performance testing utilities
pub mod performance {
    use std::time::{Instant, Duration};

    /// Measure execution time of an async operation
    pub async fn measure_execution_time<F, T>(operation: F) -> (T, Duration)
    where
        F: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = operation.await;
        let elapsed = start.elapsed();
        (result, elapsed)
    }

    /// Assert that an operation completes within a time limit
    pub async fn assert_operation_within_time_limit<F, T>(
        operation: F,
        max_duration: Duration,
    ) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let (result, elapsed) = measure_execution_time(operation).await;
        assert!(
            elapsed <= max_duration,
            "Operation took {:?}, expected <= {:?}",
            elapsed,
            max_duration
        );
        result
    }

    /// Benchmark multiple operations and return statistics
    pub async fn benchmark_operations<F, T>(
        operation: F,
        iterations: usize,
    ) -> (Vec<Duration>, Duration, Duration, Duration)
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send>>,
    {
        let mut durations = Vec::with_capacity(iterations);
        
        for _ in 0..iterations {
            let (_, duration) = measure_execution_time(operation()).await;
            durations.push(duration);
        }
        
        durations.sort();
        
        let min = durations[0];
        let max = durations[iterations - 1];
        let avg = durations.iter().sum::<Duration>() / iterations as u32;
        
        (durations, min, max, avg)
    }
}

/// Error testing utilities
pub mod error_testing {
    use crate::error::DeltaLakeError;

    /// Assert that an error is of a specific type
    pub fn assert_error_type<T>(result: Result<T, DeltaLakeError>, expected_category: &str) {
        match result {
            Ok(_) => panic!("Expected error but got Ok"),
            Err(error) => {
                assert_eq!(
                    error.category(),
                    expected_category,
                    "Expected error category '{}', got '{}'",
                    expected_category,
                    error.category()
                );
            }
        }
    }

    /// Assert that an error is retryable
    pub fn assert_error_retryable<T>(result: Result<T, DeltaLakeError>) {
        match result {
            Ok(_) => panic!("Expected error but got Ok"),
            Err(error) => {
                assert!(
                    error.is_retryable(),
                    "Expected retryable error, got non-retryable: {:?}",
                    error
                );
            }
        }
    }

    /// Assert that an error is transient
    pub fn assert_error_transient<T>(result: Result<T, DeltaLakeError>) {
        match result {
            Ok(_) => panic!("Expected error but got Ok"),
            Err(error) => {
                assert!(
                    error.is_transient(),
                    "Expected transient error, got non-transient: {:?}",
                    error
                );
            }
        }
    }
}
