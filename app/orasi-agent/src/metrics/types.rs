//! Metrics types and data structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Get current timestamp in milliseconds
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Agent metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Task processing metrics
    pub tasks: TaskMetrics,

    /// Resource usage metrics
    pub resources: ResourceMetrics,

    /// Performance metrics
    pub performance: PerformanceMetrics,

    /// Error metrics
    pub errors: ErrorMetrics,

    /// Timestamp
    pub timestamp: u64,
}

/// Task processing metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    /// Total tasks processed
    pub total_processed: u64,

    /// Tasks processed successfully
    pub successful: u64,

    /// Tasks failed
    pub failed: u64,

    /// Tasks currently pending
    pub pending: u64,

    /// Tasks currently active
    pub active: u64,

    /// Average processing time in milliseconds
    pub avg_processing_time_ms: f64,

    /// Tasks by type
    pub by_type: HashMap<String, u64>,
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// CPU usage percentage
    pub cpu_percent: f64,

    /// Memory usage in bytes
    pub memory_bytes: u64,

    /// Memory usage percentage
    pub memory_percent: f64,

    /// Disk usage in bytes
    pub disk_bytes: u64,

    /// Disk usage percentage
    pub disk_percent: f64,

    /// Network bytes received
    pub network_rx_bytes: u64,

    /// Network bytes transmitted
    pub network_tx_bytes: u64,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Requests per second
    pub requests_per_second: f64,

    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,

    /// 95th percentile response time
    pub p95_response_time_ms: f64,

    /// 99th percentile response time
    pub p99_response_time_ms: f64,

    /// Throughput in bytes per second
    pub throughput_bytes_per_sec: f64,
}

/// Error metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    /// Total errors
    pub total_errors: u64,

    /// Errors by type
    pub by_type: HashMap<String, u64>,

    /// Error rate (errors per second)
    pub error_rate: f64,
}

impl Default for AgentMetrics {
    fn default() -> Self {
        Self {
            tasks: TaskMetrics::default(),
            resources: ResourceMetrics::default(),
            performance: PerformanceMetrics::default(),
            errors: ErrorMetrics::default(),
            timestamp: current_timestamp(),
        }
    }
}

impl Default for TaskMetrics {
    fn default() -> Self {
        Self {
            total_processed: 0,
            successful: 0,
            failed: 0,
            pending: 0,
            active: 0,
            avg_processing_time_ms: 0.0,
            by_type: HashMap::new(),
        }
    }
}

impl Default for ResourceMetrics {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_bytes: 0,
            memory_percent: 0.0,
            disk_bytes: 0,
            disk_percent: 0.0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            requests_per_second: 0.0,
            avg_response_time_ms: 0.0,
            p95_response_time_ms: 0.0,
            p99_response_time_ms: 0.0,
            throughput_bytes_per_sec: 0.0,
        }
    }
}

impl Default for ErrorMetrics {
    fn default() -> Self {
        Self {
            total_errors: 0,
            by_type: HashMap::new(),
            error_rate: 0.0,
        }
    }
}
