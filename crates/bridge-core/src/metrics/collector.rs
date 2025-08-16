//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Metrics collector for the OpenTelemetry Data Lake Bridge
//!
//! This module provides the main metrics collector implementation.

use crate::error::{BridgeError, BridgeResult};
use crate::types::{ExportResult, ProcessedBatch, TelemetryBatch};
use metrics::{counter, gauge, histogram};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use super::config::MetricsConfig;

/// Bridge metrics collector
#[derive(Debug, Clone)]
pub struct BridgeMetrics {
    /// Metrics registry
    registry: Arc<RwLock<HashMap<String, MetricValue>>>,

    /// Start time
    start_time: Instant,

    /// Configuration
    config: MetricsConfig,
}

/// Metric value types
#[derive(Debug, Clone)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
}

impl BridgeMetrics {
    /// Create new metrics collector
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            registry: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
            config,
        }
    }

    /// Initialize metrics collector
    pub async fn init(&self) -> BridgeResult<()> {
        if !self.config.enabled {
            info!("Metrics collection is disabled");
            return Ok(());
        }

        info!("Initializing metrics collector");

        // Initialize metrics with current API
        counter!("bridge_initialized_total", 1);
        gauge!("bridge_uptime_seconds", 0.0);

        info!("Metrics collector initialized successfully");
        Ok(())
    }

    /// Record processed records
    pub async fn record_processed_records(&self, count: u64) {
        if !self.config.enabled {
            return;
        }

        counter!("bridge_records_processed_total", count);

        // Also update internal registry for compatibility
        let mut registry = self.registry.write().await;
        let key = "bridge_records_processed_total".to_string();

        let current_value = registry
            .get(&key)
            .and_then(|v| {
                if let MetricValue::Counter(c) = v {
                    Some(*c)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        registry.insert(key, MetricValue::Counter(current_value + count));
    }

    /// Record processed batches
    pub async fn record_processed_batches(&self, count: u64) {
        if !self.config.enabled {
            return;
        }

        counter!("bridge_batches_processed_total", count);

        // Also update internal registry for compatibility
        let mut registry = self.registry.write().await;
        let key = "bridge_batches_processed_total".to_string();

        let current_value = registry
            .get(&key)
            .and_then(|v| {
                if let MetricValue::Counter(c) = v {
                    Some(*c)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        registry.insert(key, MetricValue::Counter(current_value + count));
    }

    /// Record processing errors
    pub async fn record_processing_errors(&self, count: u64) {
        if !self.config.enabled {
            return;
        }

        counter!("bridge_processing_errors_total", count);

        // Also update internal registry for compatibility
        let mut registry = self.registry.write().await;
        let key = "bridge_processing_errors_total".to_string();

        let current_value = registry
            .get(&key)
            .and_then(|v| {
                if let MetricValue::Counter(c) = v {
                    Some(*c)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        registry.insert(key, MetricValue::Counter(current_value + count));
    }

    /// Record export errors
    pub async fn record_export_errors(&self, count: u64) {
        if !self.config.enabled {
            return;
        }

        counter!("bridge_export_errors_total", count);

        // Also update internal registry for compatibility
        let mut registry = self.registry.write().await;
        let key = "bridge_export_errors_total".to_string();

        let current_value = registry
            .get(&key)
            .and_then(|v| {
                if let MetricValue::Counter(c) = v {
                    Some(*c)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        registry.insert(key, MetricValue::Counter(current_value + count));
    }

    /// Record ingestion errors
    pub async fn record_ingestion_errors(&self, count: u64) {
        if !self.config.enabled {
            return;
        }

        counter!("bridge_ingestion_errors_total", count);

        // Also update internal registry for compatibility
        let mut registry = self.registry.write().await;
        let key = "bridge_ingestion_errors_total".to_string();

        let current_value = registry
            .get(&key)
            .and_then(|v| {
                if let MetricValue::Counter(c) = v {
                    Some(*c)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        registry.insert(key, MetricValue::Counter(current_value + count));
    }

    /// Record processing duration
    pub async fn record_processing_duration(&self, duration: Duration) {
        if !self.config.enabled {
            return;
        }

        let duration_secs = duration.as_secs_f64();
        histogram!("bridge_batch_processing_duration_seconds", duration_secs);

        // Also update internal registry for compatibility
        let mut registry = self.registry.write().await;
        let key = "bridge_batch_processing_duration_seconds".to_string();

        if let Some(MetricValue::Histogram(values)) = registry.get_mut(&key) {
            values.push(duration_secs);
        } else {
            registry.insert(key, MetricValue::Histogram(vec![duration_secs]));
        }
    }

    /// Record export duration
    pub async fn record_export_duration(&self, duration: Duration) {
        if !self.config.enabled {
            return;
        }

        let duration_secs = duration.as_secs_f64();
        histogram!("bridge_export_duration_seconds", duration_secs);

        // Also update internal registry for compatibility
        let mut registry = self.registry.write().await;
        let key = "bridge_export_duration_seconds".to_string();

        if let Some(MetricValue::Histogram(values)) = registry.get_mut(&key) {
            values.push(duration_secs);
        } else {
            registry.insert(key, MetricValue::Histogram(vec![duration_secs]));
        }
    }

    /// Update uptime gauge
    pub async fn update_uptime(&self) {
        if !self.config.enabled {
            return;
        }

        let uptime = self.start_time.elapsed().as_secs_f64();
        gauge!("bridge_uptime_seconds", uptime);
    }

    /// Record receiver stats
    pub async fn record_receiver_stats(&self, receiver_name: &str, records: u64, errors: u64) {
        if !self.config.enabled {
            return;
        }

        counter!("bridge_receiver_records_total", records, "receiver" => receiver_name.to_string());
        counter!("bridge_receiver_errors_total", errors, "receiver" => receiver_name.to_string());

        // Also update internal registry for compatibility
        let mut registry = self.registry.write().await;
        let records_key = format!("bridge_receiver_records_total_{}", receiver_name);
        let errors_key = format!("bridge_receiver_errors_total_{}", receiver_name);

        registry.insert(records_key, MetricValue::Counter(records));
        registry.insert(errors_key, MetricValue::Counter(errors));
    }

    /// Record exporter stats
    pub async fn record_exporter_stats(&self, exporter_name: &str, records: u64, errors: u64) {
        if !self.config.enabled {
            return;
        }

        counter!("bridge_exporter_records_total", records, "exporter" => exporter_name.to_string());
        counter!("bridge_exporter_errors_total", errors, "exporter" => exporter_name.to_string());

        // Also update internal registry for compatibility
        let mut registry = self.registry.write().await;
        let records_key = format!("bridge_exporter_records_total_{}", exporter_name);
        let errors_key = format!("bridge_exporter_errors_total_{}", exporter_name);

        registry.insert(records_key, MetricValue::Counter(records));
        registry.insert(errors_key, MetricValue::Counter(errors));
    }

    /// Get metrics snapshot
    pub async fn get_metrics_snapshot(&self) -> BridgeResult<HashMap<String, MetricValue>> {
        let registry = self.registry.read().await;
        Ok(registry.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_initialization() {
        let config = MetricsConfig::default();
        let metrics = BridgeMetrics::new(config);

        let result = metrics.init().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        let mut config = MetricsConfig::default();
        config.enabled = true;
        let metrics = BridgeMetrics::new(config);

        metrics.record_processed_records(10).await;
        metrics.record_processed_batches(5).await;
        metrics.record_processing_errors(2).await;

        let snapshot = metrics.get_metrics_snapshot().await.unwrap();
        assert!(!snapshot.is_empty());
    }

    #[tokio::test]
    async fn test_disabled_metrics() {
        let mut config = MetricsConfig::default();
        config.enabled = false;
        let metrics = BridgeMetrics::new(config);

        // Should not panic or error when metrics are disabled
        metrics.record_processed_records(10).await;
        metrics.record_processed_batches(5).await;

        let snapshot = metrics.get_metrics_snapshot().await.unwrap();
        assert!(snapshot.is_empty());
    }
}
