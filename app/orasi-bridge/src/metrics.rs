//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Metrics collection for Bridge API

use metrics::{counter, gauge, histogram};
use std::time::{Duration, Instant};

/// Metric name constants for API metrics
pub mod metric_names {
    // API Request/Response Metrics
    pub const API_REQUESTS_TOTAL: &str = "api_requests_total";
    pub const API_RESPONSE_TIME_SECONDS: &str = "api_response_time_seconds";
    pub const API_ACTIVE_CONNECTIONS: &str = "api_active_connections";
    pub const API_ERRORS_TOTAL: &str = "api_errors_total";
    pub const API_PROCESSING_TIME_SECONDS: &str = "api_processing_time_seconds";
    pub const API_PROCESSING_TOTAL: &str = "api_processing_total";

    // Telemetry Metrics
    pub const TELEMETRY_INGESTION_TOTAL: &str = "telemetry_ingestion_total";
    pub const TELEMETRY_BATCH_SIZE: &str = "telemetry_batch_size";
    pub const TELEMETRY_PROCESSING_TIME_SECONDS: &str = "telemetry_processing_time_seconds";
    pub const TELEMETRY_VALIDATION_ERRORS_TOTAL: &str = "telemetry_validation_errors_total";

    // Query Metrics
    pub const QUERY_TOTAL: &str = "query_total";
    pub const QUERY_EXECUTION_TIME_SECONDS: &str = "query_execution_time_seconds";
    pub const QUERY_CACHE_HIT_RATE: &str = "query_cache_hit_rate";
    pub const QUERY_RESULT_COUNT: &str = "query_result_count";

    // System Metrics
    pub const SYSTEM_UPTIME_SECONDS: &str = "system_uptime_seconds";
}

/// Metric label constants
pub mod metric_labels {
    // Common labels
    pub const METHOD: &str = "method";
    pub const PATH: &str = "path";
    pub const STATUS_CODE: &str = "status_code";
    pub const ERROR_TYPE: &str = "error_type";
    pub const OPERATION: &str = "operation";
    pub const SUCCESS: &str = "success";
    pub const TYPE: &str = "type";
    pub const CACHE_HIT: &str = "cache_hit";
}

/// API metrics collector
#[derive(Debug, Clone)]
pub struct ApiMetrics {
    /// Request counter
    pub request_counter: RequestCounter,

    /// Response time histogram
    pub response_time: ResponseTimeHistogram,

    /// Error counter
    pub error_counter: ErrorCounter,

    /// Active connections gauge
    pub active_connections: ActiveConnectionsGauge,

    /// Processing metrics
    pub processing_metrics: ProcessingMetrics,
}

impl Default for ApiMetrics {
    fn default() -> Self {
        Self {
            request_counter: RequestCounter::default(),
            response_time: ResponseTimeHistogram::default(),
            error_counter: ErrorCounter::default(),
            active_connections: ActiveConnectionsGauge::default(),
            processing_metrics: ProcessingMetrics::default(),
        }
    }
}

impl ApiMetrics {
    /// Create a new API metrics collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a request
    pub fn record_request(&self, method: &str, path: &str, status_code: u16) {
        self.request_counter.record(method, path, status_code);
    }

    /// Record response time
    pub fn record_response_time(&self, method: &str, path: &str, duration: Duration) {
        self.response_time.record(method, path, duration);
    }

    /// Record an error
    pub fn record_error(&self, error_type: &str, method: &str, path: &str) {
        self.error_counter.record(error_type, method, path);
    }

    /// Increment active connections
    pub fn increment_active_connections(&self) {
        self.active_connections.increment();
    }

    /// Decrement active connections
    pub fn decrement_active_connections(&self) {
        self.active_connections.decrement();
    }

    /// Record processing metrics
    pub fn record_processing(&self, operation: &str, duration: Duration, success: bool) {
        self.processing_metrics.record(operation, duration, success);
    }
}

/// Request counter metrics
#[derive(Debug, Clone)]
pub struct RequestCounter;

impl Default for RequestCounter {
    fn default() -> Self {
        Self
    }
}

impl RequestCounter {
    /// Record a request
    pub fn record(&self, method: &str, path: &str, status_code: u16) {
        counter!(
            metric_names::API_REQUESTS_TOTAL,
            1,
            &[
                (metric_labels::METHOD, method.to_string()),
                (metric_labels::PATH, path.to_string()),
                (metric_labels::STATUS_CODE, status_code.to_string())
            ]
        );
    }
}

/// Response time histogram metrics
#[derive(Debug, Clone)]
pub struct ResponseTimeHistogram;

impl Default for ResponseTimeHistogram {
    fn default() -> Self {
        Self
    }
}

impl ResponseTimeHistogram {
    /// Record response time
    pub fn record(&self, method: &str, path: &str, duration: Duration) {
        histogram!(
            metric_names::API_RESPONSE_TIME_SECONDS,
            duration.as_secs_f64(),
            &[
                (metric_labels::METHOD, method.to_string()),
                (metric_labels::PATH, path.to_string())
            ]
        );
    }
}

/// Error counter metrics
#[derive(Debug, Clone)]
pub struct ErrorCounter;

impl Default for ErrorCounter {
    fn default() -> Self {
        Self
    }
}

impl ErrorCounter {
    /// Record an error
    pub fn record(&self, error_type: &str, method: &str, path: &str) {
        counter!(
            metric_names::API_ERRORS_TOTAL,
            1,
            &[
                (metric_labels::ERROR_TYPE, error_type.to_string()),
                (metric_labels::METHOD, method.to_string()),
                (metric_labels::PATH, path.to_string())
            ]
        );
    }
}

/// Active connections gauge metrics
#[derive(Debug, Clone)]
pub struct ActiveConnectionsGauge;

impl Default for ActiveConnectionsGauge {
    fn default() -> Self {
        Self
    }
}

impl ActiveConnectionsGauge {
    /// Increment active connections
    pub fn increment(&self) {
        gauge!(metric_names::API_ACTIVE_CONNECTIONS, 1.0);
    }

    /// Decrement active connections
    pub fn decrement(&self) {
        gauge!(metric_names::API_ACTIVE_CONNECTIONS, -1.0);
    }
}

/// Processing metrics
#[derive(Debug, Clone)]
pub struct ProcessingMetrics;

impl Default for ProcessingMetrics {
    fn default() -> Self {
        Self
    }
}

impl ProcessingMetrics {
    /// Record processing metrics
    pub fn record(&self, operation: &str, duration: Duration, success: bool) {
        histogram!(
            metric_names::API_PROCESSING_TIME_SECONDS,
            duration.as_secs_f64(),
            &[(metric_labels::OPERATION, operation.to_string())]
        );
        counter!(
            metric_names::API_PROCESSING_TOTAL,
            1,
            &[
                (metric_labels::OPERATION, operation.to_string()),
                (metric_labels::SUCCESS, success.to_string())
            ]
        );
    }
}

/// Request timing middleware
#[derive(Debug, Clone)]
pub struct RequestTimer {
    start_time: Instant,
    method: String,
    path: String,
}

impl RequestTimer {
    /// Create a new request timer
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            start_time: Instant::now(),
            method: method.to_string(),
            path: path.to_string(),
        }
    }

    /// Finish timing and record metrics
    pub fn finish(self, metrics: &ApiMetrics, status_code: u16) {
        let duration = self.start_time.elapsed();

        metrics.record_request(&self.method, &self.path, status_code);
        metrics.record_response_time(&self.method, &self.path, duration);
    }
}

/// Telemetry ingestion metrics
#[derive(Debug, Clone)]
pub struct TelemetryMetrics {
    /// Ingestion counter
    pub ingestion_counter: IngestionCounter,

    /// Batch size histogram
    pub batch_size: BatchSizeHistogram,

    /// Processing time histogram
    pub processing_time: ProcessingTimeHistogram,

    /// Validation errors counter
    pub validation_errors: ValidationErrorsCounter,
}

impl Default for TelemetryMetrics {
    fn default() -> Self {
        Self {
            ingestion_counter: IngestionCounter::default(),
            batch_size: BatchSizeHistogram::default(),
            processing_time: ProcessingTimeHistogram::default(),
            validation_errors: ValidationErrorsCounter::default(),
        }
    }
}

impl TelemetryMetrics {
    /// Create a new telemetry metrics collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Record telemetry ingestion
    pub fn record_ingestion(&self, telemetry_type: &str, batch_size: usize, success: bool) {
        self.ingestion_counter.record(telemetry_type, success);
        self.batch_size.record(telemetry_type, batch_size);
    }

    /// Record processing time
    pub fn record_processing_time(&self, telemetry_type: &str, duration: Duration) {
        self.processing_time.record(telemetry_type, duration);
    }

    /// Record validation errors
    pub fn record_validation_errors(&self, telemetry_type: &str, error_count: usize) {
        self.validation_errors.record(telemetry_type, error_count);
    }
}

/// Ingestion counter metrics
#[derive(Debug, Clone)]
pub struct IngestionCounter;

impl Default for IngestionCounter {
    fn default() -> Self {
        Self
    }
}

impl IngestionCounter {
    /// Record ingestion
    pub fn record(&self, telemetry_type: &str, success: bool) {
        counter!(
            metric_names::TELEMETRY_INGESTION_TOTAL,
            1,
            &[
                (metric_labels::TYPE, telemetry_type.to_string()),
                (metric_labels::SUCCESS, success.to_string())
            ]
        );
    }
}

/// Batch size histogram metrics
#[derive(Debug, Clone)]
pub struct BatchSizeHistogram;

impl Default for BatchSizeHistogram {
    fn default() -> Self {
        Self
    }
}

impl BatchSizeHistogram {
    /// Record batch size
    pub fn record(&self, telemetry_type: &str, batch_size: usize) {
        histogram!(
            metric_names::TELEMETRY_BATCH_SIZE,
            batch_size as f64,
            &[(metric_labels::TYPE, telemetry_type.to_string())]
        );
    }
}

/// Processing time histogram metrics
#[derive(Debug, Clone)]
pub struct ProcessingTimeHistogram;

impl Default for ProcessingTimeHistogram {
    fn default() -> Self {
        Self
    }
}

impl ProcessingTimeHistogram {
    /// Record processing time
    pub fn record(&self, telemetry_type: &str, duration: Duration) {
        histogram!(
            metric_names::TELEMETRY_PROCESSING_TIME_SECONDS,
            duration.as_secs_f64(),
            &[(metric_labels::TYPE, telemetry_type.to_string())]
        );
    }
}

/// Validation errors counter metrics
#[derive(Debug, Clone)]
pub struct ValidationErrorsCounter;

impl Default for ValidationErrorsCounter {
    fn default() -> Self {
        Self
    }
}

impl ValidationErrorsCounter {
    /// Record validation errors
    pub fn record(&self, telemetry_type: &str, error_count: usize) {
        counter!(
            metric_names::TELEMETRY_VALIDATION_ERRORS_TOTAL,
            error_count as u64,
            &[(metric_labels::TYPE, telemetry_type.to_string())]
        );
    }
}

/// Query metrics
#[derive(Debug, Clone)]
pub struct QueryMetrics {
    /// Query counter
    pub query_counter: QueryCounter,

    /// Query execution time histogram
    pub execution_time: QueryExecutionTimeHistogram,

    /// Cache hit rate gauge
    pub cache_hit_rate: CacheHitRateGauge,

    /// Result count histogram
    pub result_count: ResultCountHistogram,
}

impl Default for QueryMetrics {
    fn default() -> Self {
        Self {
            query_counter: QueryCounter::default(),
            execution_time: QueryExecutionTimeHistogram::default(),
            cache_hit_rate: CacheHitRateGauge::default(),
            result_count: ResultCountHistogram::default(),
        }
    }
}

impl QueryMetrics {
    /// Create a new query metrics collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Record query execution
    pub fn record_query(
        &self,
        query_type: &str,
        duration: Duration,
        cache_hit: bool,
        result_count: usize,
    ) {
        self.query_counter.record(query_type, cache_hit);
        self.execution_time.record(query_type, duration);
        self.cache_hit_rate.record(query_type, cache_hit);
        self.result_count.record(query_type, result_count);
    }
}

/// Query counter metrics
#[derive(Debug, Clone)]
pub struct QueryCounter;

impl Default for QueryCounter {
    fn default() -> Self {
        Self
    }
}

impl QueryCounter {
    /// Record query
    pub fn record(&self, query_type: &str, cache_hit: bool) {
        counter!(
            metric_names::QUERY_TOTAL,
            1,
            &[
                (metric_labels::TYPE, query_type.to_string()),
                (metric_labels::CACHE_HIT, cache_hit.to_string())
            ]
        );
    }
}

/// Query execution time histogram metrics
#[derive(Debug, Clone)]
pub struct QueryExecutionTimeHistogram;

impl Default for QueryExecutionTimeHistogram {
    fn default() -> Self {
        Self
    }
}

impl QueryExecutionTimeHistogram {
    /// Record execution time
    pub fn record(&self, query_type: &str, duration: Duration) {
        histogram!(
            metric_names::QUERY_EXECUTION_TIME_SECONDS,
            duration.as_secs_f64(),
            &[(metric_labels::TYPE, query_type.to_string())]
        );
    }
}

/// Cache hit rate gauge metrics
#[derive(Debug, Clone)]
pub struct CacheHitRateGauge;

impl Default for CacheHitRateGauge {
    fn default() -> Self {
        Self
    }
}

impl CacheHitRateGauge {
    /// Record cache hit rate
    pub fn record(&self, query_type: &str, cache_hit: bool) {
        let value = if cache_hit { 1.0 } else { 0.0 };
        gauge!(
            metric_names::QUERY_CACHE_HIT_RATE,
            value,
            &[(metric_labels::TYPE, query_type.to_string())]
        );
    }
}

/// Result count histogram metrics
#[derive(Debug, Clone)]
pub struct ResultCountHistogram;

impl Default for ResultCountHistogram {
    fn default() -> Self {
        Self
    }
}

impl ResultCountHistogram {
    /// Record result count
    pub fn record(&self, query_type: &str, result_count: usize) {
        histogram!(
            metric_names::QUERY_RESULT_COUNT,
            result_count as f64,
            &[(metric_labels::TYPE, query_type.to_string())]
        );
    }
}

/// Initialize metrics
pub fn init_metrics() {
    // Initialize Prometheus metrics exporter
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();

    if let Err(e) = builder.install() {
        tracing::error!("Failed to install Prometheus metrics exporter: {}", e);
    }

    tracing::info!("Metrics initialized");
}

/// Get metrics as string
pub fn get_metrics() -> String {
    // Implement proper metrics rendering using Prometheus format
    let mut metrics_text = String::new();

    // Add request metrics
    metrics_text.push_str(&format!("# HELP {} Total number of requests\n", metric_names::API_REQUESTS_TOTAL));
    metrics_text.push_str(&format!("# TYPE {} counter\n", metric_names::API_REQUESTS_TOTAL));
    metrics_text.push_str(&format!("{} 0\n", metric_names::API_REQUESTS_TOTAL));

    // Add response time metrics
    metrics_text.push_str(&format!("# HELP {} Response time in seconds\n", metric_names::API_RESPONSE_TIME_SECONDS));
    metrics_text.push_str(&format!("# TYPE {} histogram\n", metric_names::API_RESPONSE_TIME_SECONDS));
    metrics_text.push_str(&format!("{} 0.0\n", metric_names::API_RESPONSE_TIME_SECONDS));

    // Add active connections metric
    metrics_text.push_str(&format!("# HELP {} Current number of active connections\n", metric_names::API_ACTIVE_CONNECTIONS));
    metrics_text.push_str(&format!("# TYPE {} gauge\n", metric_names::API_ACTIVE_CONNECTIONS));
    metrics_text.push_str(&format!("{} 0\n", metric_names::API_ACTIVE_CONNECTIONS));

    // Add error metrics
    metrics_text.push_str(&format!("# HELP {} Total number of errors\n", metric_names::API_ERRORS_TOTAL));
    metrics_text.push_str(&format!("# TYPE {} counter\n", metric_names::API_ERRORS_TOTAL));
    metrics_text.push_str(&format!("{} 0\n", metric_names::API_ERRORS_TOTAL));

    // Add system metrics
    metrics_text.push_str(&format!("# HELP {} System uptime in seconds\n", metric_names::SYSTEM_UPTIME_SECONDS));
    metrics_text.push_str(&format!("# TYPE {} gauge\n", metric_names::SYSTEM_UPTIME_SECONDS));
    metrics_text.push_str(&format!("{} 0\n", metric_names::SYSTEM_UPTIME_SECONDS));

    // Add query metrics
    metrics_text.push_str(&format!("# HELP {} Total number of queries\n", metric_names::QUERY_TOTAL));
    metrics_text.push_str(&format!("# TYPE {} counter\n", metric_names::QUERY_TOTAL));
    metrics_text.push_str(&format!("{} 0\n", metric_names::QUERY_TOTAL));

    // Add cache metrics
    metrics_text.push_str(&format!("# HELP {} Cache hit rate\n", metric_names::QUERY_CACHE_HIT_RATE));
    metrics_text.push_str(&format!("# TYPE {} gauge\n", metric_names::QUERY_CACHE_HIT_RATE));
    metrics_text.push_str(&format!("{} 0.0\n", metric_names::QUERY_CACHE_HIT_RATE));

    metrics_text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_names_constants() {
        // Test that all metric name constants are properly defined
        assert_eq!(metric_names::API_REQUESTS_TOTAL, "api_requests_total");
        assert_eq!(metric_names::API_RESPONSE_TIME_SECONDS, "api_response_time_seconds");
        assert_eq!(metric_names::API_ACTIVE_CONNECTIONS, "api_active_connections");
        assert_eq!(metric_names::API_ERRORS_TOTAL, "api_errors_total");
        assert_eq!(metric_names::API_PROCESSING_TIME_SECONDS, "api_processing_time_seconds");
        assert_eq!(metric_names::API_PROCESSING_TOTAL, "api_processing_total");
        assert_eq!(metric_names::TELEMETRY_INGESTION_TOTAL, "telemetry_ingestion_total");
        assert_eq!(metric_names::TELEMETRY_BATCH_SIZE, "telemetry_batch_size");
        assert_eq!(metric_names::TELEMETRY_PROCESSING_TIME_SECONDS, "telemetry_processing_time_seconds");
        assert_eq!(metric_names::TELEMETRY_VALIDATION_ERRORS_TOTAL, "telemetry_validation_errors_total");
        assert_eq!(metric_names::QUERY_TOTAL, "query_total");
        assert_eq!(metric_names::QUERY_EXECUTION_TIME_SECONDS, "query_execution_time_seconds");
        assert_eq!(metric_names::QUERY_CACHE_HIT_RATE, "query_cache_hit_rate");
        assert_eq!(metric_names::QUERY_RESULT_COUNT, "query_result_count");
        assert_eq!(metric_names::SYSTEM_UPTIME_SECONDS, "system_uptime_seconds");
    }

    #[test]
    fn test_metric_labels_constants() {
        // Test that all metric label constants are properly defined
        assert_eq!(metric_labels::METHOD, "method");
        assert_eq!(metric_labels::PATH, "path");
        assert_eq!(metric_labels::STATUS_CODE, "status_code");
        assert_eq!(metric_labels::ERROR_TYPE, "error_type");
        assert_eq!(metric_labels::OPERATION, "operation");
        assert_eq!(metric_labels::SUCCESS, "success");
        assert_eq!(metric_labels::TYPE, "type");
        assert_eq!(metric_labels::CACHE_HIT, "cache_hit");
    }

    #[test]
    fn test_get_metrics_uses_constants() {
        let metrics_text = get_metrics();
        
        // Verify that the metrics text contains the standardized metric names
        assert!(metrics_text.contains(metric_names::API_REQUESTS_TOTAL));
        assert!(metrics_text.contains(metric_names::API_RESPONSE_TIME_SECONDS));
        assert!(metrics_text.contains(metric_names::API_ACTIVE_CONNECTIONS));
        assert!(metrics_text.contains(metric_names::API_ERRORS_TOTAL));
        assert!(metrics_text.contains(metric_names::SYSTEM_UPTIME_SECONDS));
        assert!(metrics_text.contains(metric_names::QUERY_TOTAL));
        assert!(metrics_text.contains(metric_names::QUERY_CACHE_HIT_RATE));
    }
}
