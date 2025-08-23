//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Metrics collector for query performance
//!
//! This module provides metrics collection capabilities for query performance.

use async_trait::async_trait;
use bridge_core::{metrics::BridgeMetrics, BridgeResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Metrics collector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsCollectorConfig {
    /// Collector name
    pub name: String,

    /// Collector version
    pub version: String,

    /// Enable collection
    pub enable_collection: bool,

    /// Collection interval in seconds
    pub collection_interval_secs: u64,

    /// Enable detailed metrics
    pub enable_detailed_metrics: bool,

    /// Enable performance counters
    pub enable_performance_counters: bool,

    /// Enable resource monitoring
    pub enable_resource_monitoring: bool,

    /// Metrics retention period in seconds
    pub retention_period_secs: u64,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl Default for MetricsCollectorConfig {
    fn default() -> Self {
        Self {
            name: "query_metrics_collector".to_string(),
            version: "1.0.0".to_string(),
            enable_collection: true,
            collection_interval_secs: 60,
            enable_detailed_metrics: true,
            enable_performance_counters: true,
            enable_resource_monitoring: true,
            retention_period_secs: 3600, // 1 hour
            additional_config: HashMap::new(),
        }
    }
}

/// Query metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetrics {
    /// Metrics ID
    pub id: String,

    /// Metrics name
    pub name: String,

    /// Metrics description
    pub description: String,

    /// Collection timestamp
    pub timestamp: DateTime<Utc>,

    /// Query execution metrics
    pub query_metrics: QueryExecutionMetrics,

    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,

    /// Resource metrics
    pub resource_metrics: ResourceMetrics,

    /// Cache metrics
    pub cache_metrics: CacheMetrics,

    /// Error metrics
    pub error_metrics: ErrorMetrics,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Query execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExecutionMetrics {
    /// Total queries executed
    pub total_queries: u64,

    /// Successful queries
    pub successful_queries: u64,

    /// Failed queries
    pub failed_queries: u64,

    /// Queries per minute
    pub queries_per_minute: f64,

    /// Average query execution time in milliseconds
    pub avg_execution_time_ms: f64,

    /// Median query execution time in milliseconds
    pub median_execution_time_ms: f64,

    /// 95th percentile execution time in milliseconds
    pub p95_execution_time_ms: f64,

    /// 99th percentile execution time in milliseconds
    pub p99_execution_time_ms: f64,

    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,

    /// Last query execution time
    pub last_query_time: Option<DateTime<Utc>>,

    /// Active queries count
    pub active_queries: u64,

    /// Query types distribution
    pub query_types: HashMap<String, u64>,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// CPU usage percentage
    pub cpu_usage_percent: f64,

    /// Memory usage in bytes
    pub memory_usage_bytes: u64,

    /// Memory usage percentage
    pub memory_usage_percent: f64,

    /// Disk I/O read bytes
    pub disk_read_bytes: u64,

    /// Disk I/O write bytes
    pub disk_write_bytes: u64,

    /// Network I/O bytes
    pub network_io_bytes: u64,

    /// Thread count
    pub thread_count: u32,

    /// Open file descriptors
    pub open_file_descriptors: u32,

    /// Context switches per second
    pub context_switches_per_sec: f64,
}

/// Resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// Available memory in bytes
    pub available_memory_bytes: u64,

    /// Total memory in bytes
    pub total_memory_bytes: u64,

    /// Available disk space in bytes
    pub available_disk_bytes: u64,

    /// Total disk space in bytes
    pub total_disk_bytes: u64,

    /// CPU cores count
    pub cpu_cores: u32,

    /// Load average (1 minute)
    pub load_average_1m: f64,

    /// Load average (5 minutes)
    pub load_average_5m: f64,

    /// Load average (15 minutes)
    pub load_average_15m: f64,
}

/// Cache metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    /// Cache hit ratio
    pub cache_hit_ratio: f64,

    /// Cache hits
    pub cache_hits: u64,

    /// Cache misses
    pub cache_misses: u64,

    /// Cache size in bytes
    pub cache_size_bytes: u64,

    /// Cache entries count
    pub cache_entries: u64,

    /// Cache evictions
    pub cache_evictions: u64,
}

/// Error metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    /// Total errors
    pub total_errors: u64,

    /// Parse errors
    pub parse_errors: u64,

    /// Execution errors
    pub execution_errors: u64,

    /// Timeout errors
    pub timeout_errors: u64,

    /// Resource errors
    pub resource_errors: u64,

    /// Error rate per minute
    pub error_rate_per_minute: f64,

    /// Last error timestamp
    pub last_error_time: Option<DateTime<Utc>>,

    /// Error types distribution
    pub error_types: HashMap<String, u64>,
}

/// Query execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExecutionRecord {
    /// Query ID
    pub query_id: Uuid,

    /// Execution start time
    pub start_time: DateTime<Utc>,

    /// Execution end time
    pub end_time: Option<DateTime<Utc>>,

    /// Execution duration in milliseconds
    pub duration_ms: Option<u64>,

    /// Query type
    pub query_type: String,

    /// Status
    pub status: QueryExecutionStatus,

    /// Error message if failed
    pub error_message: Option<String>,

    /// Rows processed
    pub rows_processed: Option<u64>,

    /// Rows returned
    pub rows_returned: Option<u64>,
}

/// Query execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryExecutionStatus {
    Running,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

/// Metrics collector trait
#[async_trait]
pub trait MetricsCollector: Send + Sync {
    /// Collect metrics
    async fn collect(&self) -> BridgeResult<QueryMetrics>;

    /// Record query execution
    async fn record_query_execution(&self, record: QueryExecutionRecord) -> BridgeResult<()>;

    /// Record error
    async fn record_error(&self, error_type: &str, error_message: &str) -> BridgeResult<()>;

    /// Get metrics history
    async fn get_metrics_history(&self, limit: usize) -> BridgeResult<Vec<QueryMetrics>>;

    /// Clear old metrics
    async fn clear_old_metrics(&self) -> BridgeResult<()>;
}

/// Default metrics collector implementation
pub struct DefaultMetricsCollector {
    config: MetricsCollectorConfig,
    bridge_metrics: Arc<BridgeMetrics>,
    execution_records: Arc<RwLock<Vec<QueryExecutionRecord>>>,
    metrics_history: Arc<RwLock<Vec<QueryMetrics>>>,
    start_time: Instant,
}

impl DefaultMetricsCollector {
    /// Create a new metrics collector
    pub fn new(config: MetricsCollectorConfig, bridge_metrics: Arc<BridgeMetrics>) -> Self {
        Self {
            config,
            bridge_metrics,
            execution_records: Arc::new(RwLock::new(Vec::new())),
            metrics_history: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
        }
    }

    /// Calculate execution time percentiles
    fn calculate_percentiles(&self, durations: &[u64], percentile: f64) -> f64 {
        if durations.is_empty() {
            return 0.0;
        }

        let mut sorted = durations.to_vec();
        sorted.sort_unstable();

        let index = (percentile / 100.0 * (sorted.len() - 1) as f64).round() as usize;
        sorted[index] as f64
    }

    /// Get system resource metrics
    async fn get_system_metrics(&self) -> (PerformanceMetrics, ResourceMetrics) {
        // In a real implementation, you would use sysinfo or similar
        // For now, we'll return mock data
        let performance_metrics = PerformanceMetrics {
            cpu_usage_percent: 25.0,
            memory_usage_bytes: 1024 * 1024 * 512, // 512 MB
            memory_usage_percent: 50.0,
            disk_read_bytes: 1024 * 1024 * 100,  // 100 MB
            disk_write_bytes: 1024 * 1024 * 50,  // 50 MB
            network_io_bytes: 1024 * 1024 * 200, // 200 MB
            thread_count: 8,
            open_file_descriptors: 100,
            context_switches_per_sec: 1000.0,
        };

        let resource_metrics = ResourceMetrics {
            available_memory_bytes: 1024 * 1024 * 1024,     // 1 GB
            total_memory_bytes: 1024 * 1024 * 1024 * 2,     // 2 GB
            available_disk_bytes: 1024 * 1024 * 1024 * 100, // 100 GB
            total_disk_bytes: 1024 * 1024 * 1024 * 500,     // 500 GB
            cpu_cores: 4,
            load_average_1m: 0.5,
            load_average_5m: 0.3,
            load_average_15m: 0.2,
        };

        (performance_metrics, resource_metrics)
    }

    /// Get cache metrics
    async fn get_cache_metrics(&self) -> CacheMetrics {
        // In a real implementation, you would query the actual cache
        CacheMetrics {
            cache_hit_ratio: 0.85,
            cache_hits: 850,
            cache_misses: 150,
            cache_size_bytes: 1024 * 1024 * 100, // 100 MB
            cache_entries: 1000,
            cache_evictions: 50,
        }
    }
}

#[async_trait]
impl MetricsCollector for DefaultMetricsCollector {
    async fn collect(&self) -> BridgeResult<QueryMetrics> {
        if !self.config.enable_collection {
            return Err(bridge_core::BridgeError::configuration(
                "Metrics collection is disabled".to_string(),
            ));
        }

        debug!("Collecting query metrics");

        // Get execution records
        let records = self.execution_records.read().await;

        // Calculate query execution metrics
        let total_queries = records.len() as u64;
        let successful_queries = records
            .iter()
            .filter(|r| matches!(r.status, QueryExecutionStatus::Completed))
            .count() as u64;
        let failed_queries = records
            .iter()
            .filter(|r| matches!(r.status, QueryExecutionStatus::Failed))
            .count() as u64;

        let completed_records: Vec<_> =
            records.iter().filter(|r| r.duration_ms.is_some()).collect();

        let durations: Vec<u64> = completed_records
            .iter()
            .map(|r| r.duration_ms.unwrap())
            .collect();

        let avg_execution_time_ms = if !durations.is_empty() {
            durations.iter().sum::<u64>() as f64 / durations.len() as f64
        } else {
            0.0
        };

        let median_execution_time_ms = self.calculate_percentiles(&durations, 50.0);
        let p95_execution_time_ms = self.calculate_percentiles(&durations, 95.0);
        let p99_execution_time_ms = self.calculate_percentiles(&durations, 99.0);

        let total_execution_time_ms = durations.iter().sum();

        // Calculate queries per minute (last minute)
        let one_minute_ago = Utc::now() - chrono::Duration::minutes(1);
        let recent_queries = records
            .iter()
            .filter(|r| r.start_time >= one_minute_ago)
            .count() as f64;
        let queries_per_minute = recent_queries;

        // Count active queries
        let active_queries = records
            .iter()
            .filter(|r| matches!(r.status, QueryExecutionStatus::Running))
            .count() as u64;

        // Get last query time
        let last_query_time = records.iter().map(|r| r.start_time).max();

        // Calculate query types distribution
        let mut query_types = HashMap::new();
        for record in records.iter() {
            *query_types.entry(record.query_type.clone()).or_insert(0) += 1;
        }

        let query_metrics = QueryExecutionMetrics {
            total_queries,
            successful_queries,
            failed_queries,
            queries_per_minute,
            avg_execution_time_ms,
            median_execution_time_ms,
            p95_execution_time_ms,
            p99_execution_time_ms,
            total_execution_time_ms,
            last_query_time,
            active_queries,
            query_types,
        };

        // Get system metrics
        let (performance_metrics, resource_metrics) = self.get_system_metrics().await;

        // Get cache metrics
        let cache_metrics = self.get_cache_metrics().await;

        // Calculate error metrics
        let error_records = records
            .iter()
            .filter(|r| matches!(r.status, QueryExecutionStatus::Failed))
            .collect::<Vec<_>>();

        let total_errors = error_records.len() as u64;
        let parse_errors = error_records
            .iter()
            .filter(|r| {
                r.error_message
                    .as_ref()
                    .map_or(false, |e| e.contains("parse"))
            })
            .count() as u64;
        let execution_errors = error_records
            .iter()
            .filter(|r| {
                r.error_message
                    .as_ref()
                    .map_or(false, |e| e.contains("execution"))
            })
            .count() as u64;
        let timeout_errors = error_records
            .iter()
            .filter(|r| matches!(r.status, QueryExecutionStatus::Timeout))
            .count() as u64;
        let resource_errors = error_records
            .iter()
            .filter(|r| {
                r.error_message
                    .as_ref()
                    .map_or(false, |e| e.contains("resource"))
            })
            .count() as u64;

        let error_rate_per_minute = error_records
            .iter()
            .filter(|r| r.start_time >= one_minute_ago)
            .count() as f64;

        let last_error_time = error_records.iter().map(|r| r.start_time).max();

        let mut error_types = HashMap::new();
        for record in error_records.iter() {
            if let Some(error_msg) = &record.error_message {
                let error_type = if error_msg.contains("parse") {
                    "parse_error"
                } else if error_msg.contains("execution") {
                    "execution_error"
                } else if error_msg.contains("timeout") {
                    "timeout_error"
                } else if error_msg.contains("resource") {
                    "resource_error"
                } else {
                    "unknown_error"
                };
                *error_types.entry(error_type.to_string()).or_insert(0) += 1;
            }
        }

        let error_metrics = ErrorMetrics {
            total_errors,
            parse_errors,
            execution_errors,
            timeout_errors,
            resource_errors,
            error_rate_per_minute,
            last_error_time,
            error_types,
        };

        // Create metrics object
        let metrics = QueryMetrics {
            id: Uuid::new_v4().to_string(),
            name: self.config.name.clone(),
            description: "Query engine performance metrics".to_string(),
            timestamp: Utc::now(),
            query_metrics,
            performance_metrics,
            resource_metrics,
            cache_metrics,
            error_metrics,
            additional_config: self.config.additional_config.clone(),
        };

        // Store in history
        {
            let mut history = self.metrics_history.write().await;
            history.push(metrics.clone());

            // Trim history if it exceeds retention period
            let retention_duration =
                chrono::Duration::seconds(self.config.retention_period_secs as i64);
            let cutoff_time = Utc::now() - retention_duration;
            history.retain(|m| m.timestamp >= cutoff_time);
        }

        // Update bridge metrics
        if self.config.enable_performance_counters {
            self.bridge_metrics
                .record_processed_records(total_queries)
                .await;
            if total_errors > 0 {
                self.bridge_metrics
                    .record_processing_errors(total_errors)
                    .await;
            }
        }

        info!(
            "Collected query metrics: {} queries, {} errors",
            total_queries, total_errors
        );

        Ok(metrics)
    }

    async fn record_query_execution(&self, record: QueryExecutionRecord) -> BridgeResult<()> {
        if !self.config.enable_collection {
            return Ok(());
        }

        let query_id = record.query_id;
        let mut records = self.execution_records.write().await;
        records.push(record);

        // Keep only recent records to prevent memory bloat
        let retention_duration =
            chrono::Duration::seconds(self.config.retention_period_secs as i64);
        let cutoff_time = Utc::now() - retention_duration;
        records.retain(|r| r.start_time >= cutoff_time);

        debug!("Recorded query execution: {:?}", query_id);
        Ok(())
    }

    async fn record_error(&self, error_type: &str, error_message: &str) -> BridgeResult<()> {
        if !self.config.enable_collection {
            return Ok(());
        }

        let record = QueryExecutionRecord {
            query_id: Uuid::new_v4(),
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            duration_ms: Some(0),
            query_type: "error".to_string(),
            status: QueryExecutionStatus::Failed,
            error_message: Some(format!("{}: {}", error_type, error_message)),
            rows_processed: None,
            rows_returned: None,
        };

        self.record_query_execution(record).await?;

        // Update bridge metrics
        if self.config.enable_performance_counters {
            self.bridge_metrics.record_processing_errors(1).await;
        }

        debug!("Recorded error: {} - {}", error_type, error_message);
        Ok(())
    }

    async fn get_metrics_history(&self, limit: usize) -> BridgeResult<Vec<QueryMetrics>> {
        let history = self.metrics_history.read().await;
        let mut result = history.clone();
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        result.truncate(limit);
        Ok(result)
    }

    async fn clear_old_metrics(&self) -> BridgeResult<()> {
        let retention_duration =
            chrono::Duration::seconds(self.config.retention_period_secs as i64);
        let cutoff_time = Utc::now() - retention_duration;

        // Clear old execution records
        {
            let mut records = self.execution_records.write().await;
            records.retain(|r| r.start_time >= cutoff_time);
        }

        // Clear old metrics history
        {
            let mut history = self.metrics_history.write().await;
            history.retain(|m| m.timestamp >= cutoff_time);
        }

        info!(
            "Cleared old metrics older than {} seconds",
            self.config.retention_period_secs
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_core::metrics::MetricsConfig;

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let config = MetricsCollectorConfig::default();
        let bridge_metrics = Arc::new(BridgeMetrics::new(MetricsConfig::default()));
        let collector = DefaultMetricsCollector::new(config, bridge_metrics);

        assert_eq!(collector.config.name, "query_metrics_collector");
        assert!(collector.config.enable_collection);
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        let config = MetricsCollectorConfig::default();
        let bridge_metrics = Arc::new(BridgeMetrics::new(MetricsConfig::default()));
        let collector = DefaultMetricsCollector::new(config, bridge_metrics);

        let metrics = collector.collect().await.unwrap();
        assert_eq!(metrics.name, "query_metrics_collector");
        assert_eq!(metrics.query_metrics.total_queries, 0);
    }

    #[tokio::test]
    async fn test_query_execution_recording() {
        let config = MetricsCollectorConfig::default();
        let bridge_metrics = Arc::new(BridgeMetrics::new(MetricsConfig::default()));
        let collector = DefaultMetricsCollector::new(config, bridge_metrics);

        let record = QueryExecutionRecord {
            query_id: Uuid::new_v4(),
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            duration_ms: Some(100),
            query_type: "SELECT".to_string(),
            status: QueryExecutionStatus::Completed,
            error_message: None,
            rows_processed: Some(1000),
            rows_returned: Some(100),
        };

        collector.record_query_execution(record).await.unwrap();

        let metrics = collector.collect().await.unwrap();
        assert_eq!(metrics.query_metrics.total_queries, 1);
        assert_eq!(metrics.query_metrics.successful_queries, 1);
    }

    #[tokio::test]
    async fn test_error_recording() {
        let config = MetricsCollectorConfig::default();
        let bridge_metrics = Arc::new(BridgeMetrics::new(MetricsConfig::default()));
        let collector = DefaultMetricsCollector::new(config, bridge_metrics);

        collector
            .record_error("parse_error", "Invalid SQL syntax")
            .await
            .unwrap();

        let metrics = collector.collect().await.unwrap();
        assert_eq!(metrics.error_metrics.total_errors, 1);
        assert_eq!(metrics.error_metrics.parse_errors, 1);
    }

    #[tokio::test]
    async fn test_disabled_collection() {
        let mut config = MetricsCollectorConfig::default();
        config.enable_collection = false;
        let bridge_metrics = Arc::new(BridgeMetrics::new(MetricsConfig::default()));
        let collector = DefaultMetricsCollector::new(config, bridge_metrics);

        let result = collector.collect().await;
        assert!(result.is_err());
    }
}
