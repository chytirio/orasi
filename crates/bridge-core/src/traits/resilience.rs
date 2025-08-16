//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Resilience traits for the OpenTelemetry Data Lake Bridge
//!
//! This module provides traits and types for fault tolerance and resilience
//! patterns including circuit breakers and retry policies.

use crate::error::BridgeResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Circuit breaker trait for fault tolerance
#[async_trait]
pub trait CircuitBreaker: Send + Sync {
    /// Check if circuit breaker is open
    fn is_open(&self) -> bool;

    /// Check if circuit breaker is half-open
    fn is_half_open(&self) -> bool;

    /// Check if circuit breaker is closed
    fn is_closed(&self) -> bool;

    /// Record success
    async fn record_success(&self) -> BridgeResult<()>;

    /// Record failure
    async fn record_failure(&self) -> BridgeResult<()>;

    /// Get circuit breaker statistics
    async fn get_stats(&self) -> BridgeResult<CircuitBreakerStats>;

    /// Reset circuit breaker
    async fn reset(&self) -> BridgeResult<()>;
}

/// Circuit breaker statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerStats {
    /// Current state
    pub state: CircuitBreakerState,

    /// Total requests
    pub total_requests: u64,

    /// Successful requests
    pub successful_requests: u64,

    /// Failed requests
    pub failed_requests: u64,

    /// Success rate
    pub success_rate: f64,

    /// Last state change timestamp
    pub last_state_change: Option<chrono::DateTime<chrono::Utc>>,

    /// Last request timestamp
    pub last_request_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Circuit breaker state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Retry policy trait for retry logic
#[async_trait]
pub trait RetryPolicy: Send + Sync {
    /// Check if retry should be attempted
    fn should_retry(&self, attempt: u32, error: &crate::error::BridgeError) -> bool;

    /// Get delay for next retry
    fn get_delay(&self, attempt: u32) -> std::time::Duration;

    /// Get maximum retry attempts
    fn max_attempts(&self) -> u32;

    /// Get retry policy statistics
    async fn get_stats(&self) -> BridgeResult<RetryPolicyStats>;
}

/// Retry policy statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicyStats {
    /// Total retry attempts
    pub total_retries: u64,

    /// Successful retries
    pub successful_retries: u64,

    /// Failed retries
    pub failed_retries: u64,

    /// Average retry attempts per operation
    pub avg_retry_attempts: f64,

    /// Last retry timestamp
    pub last_retry_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Metrics collector trait for collecting metrics
#[async_trait]
pub trait MetricsCollector: Send + Sync {
    /// Increment counter
    async fn increment_counter(
        &self,
        name: &str,
        value: u64,
        labels: &[(&str, &str)],
    ) -> BridgeResult<()>;

    /// Record gauge
    async fn record_gauge(
        &self,
        name: &str,
        value: f64,
        labels: &[(&str, &str)],
    ) -> BridgeResult<()>;

    /// Record histogram
    async fn record_histogram(
        &self,
        name: &str,
        value: f64,
        labels: &[(&str, &str)],
    ) -> BridgeResult<()>;

    /// Record summary
    async fn record_summary(
        &self,
        name: &str,
        value: f64,
        labels: &[(&str, &str)],
    ) -> BridgeResult<()>;

    /// Get metrics collector statistics
    async fn get_stats(&self) -> BridgeResult<MetricsCollectorStats>;
}

/// Metrics collector statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsCollectorStats {
    /// Total metrics recorded
    pub total_metrics: u64,

    /// Metrics recorded in last minute
    pub metrics_per_minute: u64,

    /// Total counters
    pub total_counters: u64,

    /// Total gauges
    pub total_gauges: u64,

    /// Total histograms
    pub total_histograms: u64,

    /// Total summaries
    pub total_summaries: u64,

    /// Last metric timestamp
    pub last_metric_time: Option<chrono::DateTime<chrono::Utc>>,
}
