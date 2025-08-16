//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Delta Lake metrics collection
//! 
//! This module provides metrics collection and monitoring capabilities
//! for the Delta Lake connector.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use metrics::{counter, gauge, histogram, register_counter, register_gauge, register_histogram};
use crate::error::{DeltaLakeError, DeltaLakeResult};

/// Delta Lake metrics configuration
#[derive(Debug, Clone)]
pub struct DeltaLakeMetricsConfig {
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Metrics collection interval
    pub collection_interval: Duration,
    /// Enable detailed metrics
    pub enable_detailed_metrics: bool,
}

impl Default for DeltaLakeMetricsConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            collection_interval: Duration::from_secs(60), // 1 minute
            enable_detailed_metrics: false,
        }
    }
}

/// Delta Lake metrics collector
pub struct DeltaLakeMetrics {
    /// Metrics configuration
    config: DeltaLakeMetricsConfig,
    /// Metrics state
    state: Arc<RwLock<DeltaLakeMetricsState>>,
}

/// Delta Lake metrics state
#[derive(Debug, Clone)]
pub struct DeltaLakeMetricsState {
    /// Total records written
    pub total_records_written: u64,
    /// Total bytes written
    pub total_bytes_written: u64,
    /// Total records read
    pub total_records_read: u64,
    /// Total bytes read
    pub total_bytes_read: u64,
    /// Total transactions
    pub total_transactions: u64,
    /// Successful transactions
    pub successful_transactions: u64,
    /// Failed transactions
    pub failed_transactions: u64,
    /// Active transactions
    pub active_transactions: u32,
    /// Write operations
    pub write_operations: u64,
    /// Read operations
    pub read_operations: u64,
    /// Schema operations
    pub schema_operations: u64,
    /// Errors
    pub errors: u64,
    /// Last write time
    pub last_write_time: Option<Instant>,
    /// Last read time
    pub last_read_time: Option<Instant>,
    /// Average write latency
    pub avg_write_latency_ms: f64,
    /// Average read latency
    pub avg_read_latency_ms: f64,
    /// Average transaction duration
    pub avg_transaction_duration_ms: f64,
}

impl Default for DeltaLakeMetricsState {
    fn default() -> Self {
        Self {
            total_records_written: 0,
            total_bytes_written: 0,
            total_records_read: 0,
            total_bytes_read: 0,
            total_transactions: 0,
            successful_transactions: 0,
            failed_transactions: 0,
            active_transactions: 0,
            write_operations: 0,
            read_operations: 0,
            schema_operations: 0,
            errors: 0,
            last_write_time: None,
            last_read_time: None,
            avg_write_latency_ms: 0.0,
            avg_read_latency_ms: 0.0,
            avg_transaction_duration_ms: 0.0,
        }
    }
}

impl DeltaLakeMetrics {
    /// Create a new Delta Lake metrics collector
    pub fn new(config: DeltaLakeMetricsConfig) -> DeltaLakeResult<Self> {
        let metrics = Self {
            config,
            state: Arc::new(RwLock::new(DeltaLakeMetricsState::default())),
        };
        
        metrics.register_metrics()?;
        Ok(metrics)
    }

    /// Register metrics with the metrics system
    fn register_metrics(&self) -> DeltaLakeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        // Register counters
        register_counter!("deltalake_records_written_total", "Total records written to Delta Lake");
        register_counter!("deltalake_bytes_written_total", "Total bytes written to Delta Lake");
        register_counter!("deltalake_records_read_total", "Total records read from Delta Lake");
        register_counter!("deltalake_bytes_read_total", "Total bytes read from Delta Lake");
        register_counter!("deltalake_transactions_total", "Total transactions");
        register_counter!("deltalake_transactions_successful_total", "Successful transactions");
        register_counter!("deltalake_transactions_failed_total", "Failed transactions");
        register_counter!("deltalake_write_operations_total", "Total write operations");
        register_counter!("deltalake_read_operations_total", "Total read operations");
        register_counter!("deltalake_schema_operations_total", "Total schema operations");
        register_counter!("deltalake_errors_total", "Total errors");

        // Register gauges
        register_gauge!("deltalake_active_transactions", "Number of active transactions");
        register_gauge!("deltalake_avg_write_latency_ms", "Average write latency in milliseconds");
        register_gauge!("deltalake_avg_read_latency_ms", "Average read latency in milliseconds");
        register_gauge!("deltalake_avg_transaction_duration_ms", "Average transaction duration in milliseconds");

        // Register histograms
        register_histogram!("deltalake_write_latency_ms", "Write operation latency in milliseconds");
        register_histogram!("deltalake_read_latency_ms", "Read operation latency in milliseconds");
        register_histogram!("deltalake_transaction_duration_ms", "Transaction duration in milliseconds");

        Ok(())
    }

    /// Record records written
    pub async fn record_records_written(&self, count: u64) -> DeltaLakeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        counter!("deltalake_records_written_total", count);
        
        let mut state = self.state.write().await;
        state.total_records_written += count;
        state.write_operations += 1;
        state.last_write_time = Some(Instant::now());
        
        Ok(())
    }

    /// Record bytes written
    pub async fn record_bytes_written(&self, bytes: u64) -> DeltaLakeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        counter!("deltalake_bytes_written_total", bytes);
        
        let mut state = self.state.write().await;
        state.total_bytes_written += bytes;
        
        Ok(())
    }

    /// Record records read
    pub async fn record_records_read(&self, count: u64) -> DeltaLakeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        counter!("deltalake_records_read_total", count);
        
        let mut state = self.state.write().await;
        state.total_records_read += count;
        state.read_operations += 1;
        state.last_read_time = Some(Instant::now());
        
        Ok(())
    }

    /// Record bytes read
    pub async fn record_bytes_read(&self, bytes: u64) -> DeltaLakeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        counter!("deltalake_bytes_read_total", bytes);
        
        let mut state = self.state.write().await;
        state.total_bytes_read += bytes;
        
        Ok(())
    }

    /// Record transaction
    pub async fn record_transaction(&self, success: bool, duration: Duration) -> DeltaLakeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        counter!("deltalake_transactions_total", 1);
        
        if success {
            counter!("deltalake_transactions_successful_total", 1);
        } else {
            counter!("deltalake_transactions_failed_total", 1);
        }

        let duration_ms = duration.as_millis() as f64;
        histogram!("deltalake_transaction_duration_ms", duration_ms);
        
        let mut state = self.state.write().await;
        state.total_transactions += 1;
        if success {
            state.successful_transactions += 1;
        } else {
            state.failed_transactions += 1;
        }
        
        // Update average transaction duration
        let total_duration = state.avg_transaction_duration_ms * (state.total_transactions - 1) as f64 + duration_ms;
        state.avg_transaction_duration_ms = total_duration / state.total_transactions as f64;
        
        gauge!("deltalake_avg_transaction_duration_ms", state.avg_transaction_duration_ms);
        
        Ok(())
    }

    /// Record active transactions
    pub async fn record_active_transactions(&self, count: u32) -> DeltaLakeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        gauge!("deltalake_active_transactions", count as f64);
        
        let mut state = self.state.write().await;
        state.active_transactions = count;
        
        Ok(())
    }

    /// Record write operation
    pub async fn record_write_operation(&self, latency: Duration) -> DeltaLakeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        counter!("deltalake_write_operations_total", 1);
        
        let latency_ms = latency.as_millis() as f64;
        histogram!("deltalake_write_latency_ms", latency_ms);
        
        let mut state = self.state.write().await;
        state.write_operations += 1;
        
        // Update average write latency
        let total_latency = state.avg_write_latency_ms * (state.write_operations - 1) as f64 + latency_ms;
        state.avg_write_latency_ms = total_latency / state.write_operations as f64;
        
        gauge!("deltalake_avg_write_latency_ms", state.avg_write_latency_ms);
        
        Ok(())
    }

    /// Record read operation
    pub async fn record_read_operation(&self, latency: Duration) -> DeltaLakeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        counter!("deltalake_read_operations_total", 1);
        
        let latency_ms = latency.as_millis() as f64;
        histogram!("deltalake_read_latency_ms", latency_ms);
        
        let mut state = self.state.write().await;
        state.read_operations += 1;
        
        // Update average read latency
        let total_latency = state.avg_read_latency_ms * (state.read_operations - 1) as f64 + latency_ms;
        state.avg_read_latency_ms = total_latency / state.read_operations as f64;
        
        gauge!("deltalake_avg_read_latency_ms", state.avg_read_latency_ms);
        
        Ok(())
    }

    /// Record schema operation
    pub async fn record_schema_operation(&self) -> DeltaLakeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        counter!("deltalake_schema_operations_total", 1);
        
        let mut state = self.state.write().await;
        state.schema_operations += 1;
        
        Ok(())
    }

    /// Record error
    pub async fn record_error(&self) -> DeltaLakeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        counter!("deltalake_errors_total", 1);
        
        let mut state = self.state.write().await;
        state.errors += 1;
        
        Ok(())
    }

    /// Get metrics state
    pub async fn get_state(&self) -> DeltaLakeMetricsState {
        self.state.read().await.clone()
    }

    /// Get metrics snapshot
    pub async fn get_snapshot(&self) -> DeltaLakeResult<DeltaLakeMetricsSnapshot> {
        let state = self.state.read().await;
        
        Ok(DeltaLakeMetricsSnapshot {
            total_records_written: state.total_records_written,
            total_bytes_written: state.total_bytes_written,
            total_records_read: state.total_records_read,
            total_bytes_read: state.total_bytes_read,
            total_transactions: state.total_transactions,
            successful_transactions: state.successful_transactions,
            failed_transactions: state.failed_transactions,
            active_transactions: state.active_transactions,
            write_operations: state.write_operations,
            read_operations: state.read_operations,
            schema_operations: state.schema_operations,
            errors: state.errors,
            avg_write_latency_ms: state.avg_write_latency_ms,
            avg_read_latency_ms: state.avg_read_latency_ms,
            avg_transaction_duration_ms: state.avg_transaction_duration_ms,
        })
    }

    /// Reset metrics
    pub async fn reset(&self) -> DeltaLakeResult<()> {
        let mut state = self.state.write().await;
        *state = DeltaLakeMetricsState::default();
        
        info!("Delta Lake metrics reset");
        Ok(())
    }
}

/// Delta Lake metrics snapshot
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeltaLakeMetricsSnapshot {
    /// Total records written
    pub total_records_written: u64,
    /// Total bytes written
    pub total_bytes_written: u64,
    /// Total records read
    pub total_records_read: u64,
    /// Total bytes read
    pub total_bytes_read: u64,
    /// Total transactions
    pub total_transactions: u64,
    /// Successful transactions
    pub successful_transactions: u64,
    /// Failed transactions
    pub failed_transactions: u64,
    /// Active transactions
    pub active_transactions: u32,
    /// Write operations
    pub write_operations: u64,
    /// Read operations
    pub read_operations: u64,
    /// Schema operations
    pub schema_operations: u64,
    /// Errors
    pub errors: u64,
    /// Average write latency
    pub avg_write_latency_ms: f64,
    /// Average read latency
    pub avg_read_latency_ms: f64,
    /// Average transaction duration
    pub avg_transaction_duration_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_creation() {
        let config = DeltaLakeMetricsConfig::default();
        let metrics = DeltaLakeMetrics::new(config).unwrap();
        
        let state = metrics.get_state().await;
        assert_eq!(state.total_records_written, 0);
        assert_eq!(state.total_transactions, 0);
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        let config = DeltaLakeMetricsConfig::default();
        let metrics = DeltaLakeMetrics::new(config).unwrap();
        
        metrics.record_records_written(100).await.unwrap();
        metrics.record_transaction(true, Duration::from_millis(50)).await.unwrap();
        
        let state = metrics.get_state().await;
        assert_eq!(state.total_records_written, 100);
        assert_eq!(state.total_transactions, 1);
        assert_eq!(state.successful_transactions, 1);
    }
}
