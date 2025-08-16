//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Comprehensive metrics collection for telemetry ingestion monitoring
//!
//! This module provides detailed metrics collection, monitoring, and observability
//! features for the telemetry ingestion system.

use async_trait::async_trait;
use bridge_core::{traits::MetricsCollector as BridgeMetricsCollector, BridgeResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Metrics collection interval
    pub collection_interval: Duration,
    /// Enable detailed metrics
    pub enable_detailed_metrics: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            collection_interval: Duration::from_secs(15),
            enable_detailed_metrics: true,
        }
    }
}

/// Ingestion metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IngestionMetrics {
    pub total_batches: u64,
    pub total_records: u64,
    pub batches_per_minute: u64,
    pub records_per_minute: u64,
    pub avg_processing_time_ms: f64,
    pub error_count: u64,
    pub last_process_time: Option<DateTime<Utc>>,
}

/// Protocol metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProtocolMetrics {
    pub protocol: String,
    pub total_messages: u64,
    pub messages_per_minute: u64,
    pub total_bytes: u64,
    pub bytes_per_minute: u64,
    pub error_count: u64,
    pub last_message_time: Option<DateTime<Utc>>,
    pub is_connected: bool,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceMetrics {
    pub total_processing_time_ms: f64,
    pub avg_batch_size: f64,
    pub max_batch_size: u64,
    pub min_batch_size: u64,
    pub total_batches_processed: u64,
    pub total_records_processed: u64,
    pub processing_throughput_records_per_sec: f64,
}

/// Error metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ErrorMetrics {
    pub total_errors: u64,
    pub errors_per_minute: u64,
    pub error_types: HashMap<String, u64>,
    pub last_error_time: Option<DateTime<Utc>>,
    pub error_rate_percentage: f64,
}

/// All metrics combined
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllMetrics {
    pub ingestion: IngestionMetrics,
    pub protocol: ProtocolMetrics,
    pub performance: PerformanceMetrics,
    pub error: ErrorMetrics,
    pub uptime_seconds: u64,
}

/// Simple metrics collector
#[derive(Clone)]
pub struct MetricsCollector {
    config: MetricsConfig,
    ingestion_metrics: Arc<RwLock<IngestionMetrics>>,
    protocol_metrics: Arc<RwLock<ProtocolMetrics>>,
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    error_metrics: Arc<RwLock<ErrorMetrics>>,
    collection_start_time: Instant,
    // Add tracking for different metric types
    counters: Arc<RwLock<HashMap<String, u64>>>,
    gauges: Arc<RwLock<HashMap<String, f64>>>,
    histograms: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    summaries: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    total_metrics: Arc<RwLock<u64>>,
    last_metric_time: Arc<RwLock<Option<DateTime<Utc>>>>,
}

#[async_trait]
impl BridgeMetricsCollector for MetricsCollector {
    async fn increment_counter(
        &self,
        name: &str,
        value: u64,
        labels: &[(&str, &str)],
    ) -> BridgeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        let metric_key = self.create_metric_key(name, labels);
        let mut counters = self.counters.write().await;
        let current_value = *counters.get(&metric_key).unwrap_or(&0);
        counters.insert(metric_key, current_value + value);

        // Update total metrics and timestamp
        {
            let mut total_metrics = self.total_metrics.write().await;
            *total_metrics += 1;
        }
        {
            let mut last_metric_time = self.last_metric_time.write().await;
            *last_metric_time = Some(Utc::now());
        }

        info!(
            "Counter incremented: {} = {} (total: {})",
            name,
            value,
            current_value + value
        );
        Ok(())
    }

    async fn record_gauge(
        &self,
        name: &str,
        value: f64,
        labels: &[(&str, &str)],
    ) -> BridgeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        let metric_key = self.create_metric_key(name, labels);
        let mut gauges = self.gauges.write().await;
        gauges.insert(metric_key, value);

        // Update total metrics and timestamp
        {
            let mut total_metrics = self.total_metrics.write().await;
            *total_metrics += 1;
        }
        {
            let mut last_metric_time = self.last_metric_time.write().await;
            *last_metric_time = Some(Utc::now());
        }

        info!("Gauge recorded: {} = {}", name, value);
        Ok(())
    }

    async fn record_histogram(
        &self,
        name: &str,
        value: f64,
        labels: &[(&str, &str)],
    ) -> BridgeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        let metric_key = self.create_metric_key(name, labels);
        let mut histograms = self.histograms.write().await;
        let values = histograms.entry(metric_key).or_insert_with(Vec::new);
        values.push(value);

        // Update total metrics and timestamp
        {
            let mut total_metrics = self.total_metrics.write().await;
            *total_metrics += 1;
        }
        {
            let mut last_metric_time = self.last_metric_time.write().await;
            *last_metric_time = Some(Utc::now());
        }

        info!(
            "Histogram recorded: {} = {} (total samples: {})",
            name,
            value,
            values.len()
        );
        Ok(())
    }

    async fn record_summary(
        &self,
        name: &str,
        value: f64,
        labels: &[(&str, &str)],
    ) -> BridgeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        let metric_key = self.create_metric_key(name, labels);
        let mut summaries = self.summaries.write().await;
        let values = summaries.entry(metric_key).or_insert_with(Vec::new);
        values.push(value);

        // Update total metrics and timestamp
        {
            let mut total_metrics = self.total_metrics.write().await;
            *total_metrics += 1;
        }
        {
            let mut last_metric_time = self.last_metric_time.write().await;
            *last_metric_time = Some(Utc::now());
        }

        info!(
            "Summary recorded: {} = {} (total samples: {})",
            name,
            value,
            values.len()
        );
        Ok(())
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::MetricsCollectorStats> {
        let total_metrics = self.total_metrics.read().await;
        let last_metric_time = self.last_metric_time.read().await;
        let counters = self.counters.read().await;
        let gauges = self.gauges.read().await;
        let histograms = self.histograms.read().await;
        let summaries = self.summaries.read().await;

        // Calculate metrics per minute (simplified - in a real implementation this would track time windows)
        let metrics_per_minute = if self.collection_start_time.elapsed().as_secs() > 60 {
            *total_metrics / (self.collection_start_time.elapsed().as_secs() / 60)
        } else {
            *total_metrics
        };

        Ok(bridge_core::traits::MetricsCollectorStats {
            total_metrics: *total_metrics,
            metrics_per_minute,
            total_counters: counters.len() as u64,
            total_gauges: gauges.len() as u64,
            total_histograms: histograms.len() as u64,
            total_summaries: summaries.len() as u64,
            last_metric_time: *last_metric_time,
        })
    }
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new(config: MetricsConfig) -> BridgeResult<Self> {
        let ingestion_metrics = Arc::new(RwLock::new(IngestionMetrics::default()));
        let protocol_metrics = Arc::new(RwLock::new(ProtocolMetrics::default()));
        let performance_metrics = Arc::new(RwLock::new(PerformanceMetrics::default()));
        let error_metrics = Arc::new(RwLock::new(ErrorMetrics::default()));
        let collection_start_time = Instant::now();

        Ok(Self {
            config,
            ingestion_metrics,
            protocol_metrics,
            performance_metrics,
            error_metrics,
            collection_start_time,
            // Add tracking for different metric types
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
            summaries: Arc::new(RwLock::new(HashMap::new())),
            total_metrics: Arc::new(RwLock::new(0)),
            last_metric_time: Arc::new(RwLock::new(None)),
        })
    }

    /// Record batch processed
    pub async fn record_batch_processed(&self, batch_size: usize, processing_time: Duration) {
        if !self.config.enable_metrics {
            return;
        }

        let mut metrics = self.ingestion_metrics.write().await;
        metrics.total_batches += 1;
        metrics.total_records += batch_size as u64;
        metrics.last_process_time = Some(Utc::now());

        // Update average processing time
        let processing_time_ms = processing_time.as_millis() as f64;
        if metrics.total_batches > 0 {
            let total_time = metrics.avg_processing_time_ms * (metrics.total_batches - 1) as f64
                + processing_time_ms;
            metrics.avg_processing_time_ms = total_time / metrics.total_batches as f64;
        }
    }

    /// Record error
    pub async fn record_error(&self, error_type: &str) {
        if !self.config.enable_metrics {
            return;
        }

        let mut error_metrics = self.error_metrics.write().await;
        error_metrics.total_errors += 1;
        error_metrics.last_error_time = Some(Utc::now());

        let count = error_metrics
            .error_types
            .entry(error_type.to_string())
            .or_insert(0);
        *count += 1;
    }

    /// Get all metrics
    pub async fn get_all_metrics(&self) -> AllMetrics {
        let ingestion_metrics = self.ingestion_metrics.read().await.clone();
        let protocol_metrics = self.protocol_metrics.read().await.clone();
        let performance_metrics = self.performance_metrics.read().await.clone();
        let error_metrics = self.error_metrics.read().await.clone();

        AllMetrics {
            ingestion: ingestion_metrics,
            protocol: protocol_metrics,
            performance: performance_metrics,
            error: error_metrics,
            uptime_seconds: self.collection_start_time.elapsed().as_secs(),
        }
    }

    /// Start metrics collection
    pub async fn start_collection(&self) -> BridgeResult<()> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        info!(
            "Starting metrics collection with interval: {:?}",
            self.config.collection_interval
        );

        // For now, just log that collection is started
        // In a real implementation, this would spawn a background task
        Ok(())
    }

    /// Stop metrics collection
    pub async fn stop_collection(&self) -> BridgeResult<()> {
        info!("Stopping metrics collection");
        Ok(())
    }

    /// Create a metric key from name and labels
    fn create_metric_key(&self, name: &str, labels: &[(&str, &str)]) -> String {
        if labels.is_empty() {
            return name.to_string();
        }

        let mut key = name.to_string();
        key.push('{');

        for (i, (label_name, label_value)) in labels.iter().enumerate() {
            if i > 0 {
                key.push(',');
            }
            key.push_str(label_name);
            key.push('=');
            key.push_str(label_value);
        }

        key.push('}');
        key
    }

    /// Get counter value
    pub async fn get_counter(&self, name: &str, labels: &[(&str, &str)]) -> Option<u64> {
        let metric_key = self.create_metric_key(name, labels);
        let counters = self.counters.read().await;
        counters.get(&metric_key).copied()
    }

    /// Get gauge value
    pub async fn get_gauge(&self, name: &str, labels: &[(&str, &str)]) -> Option<f64> {
        let metric_key = self.create_metric_key(name, labels);
        let gauges = self.gauges.read().await;
        gauges.get(&metric_key).copied()
    }

    /// Get histogram values
    pub async fn get_histogram(&self, name: &str, labels: &[(&str, &str)]) -> Option<Vec<f64>> {
        let metric_key = self.create_metric_key(name, labels);
        let histograms = self.histograms.read().await;
        histograms.get(&metric_key).cloned()
    }

    /// Get summary values
    pub async fn get_summary(&self, name: &str, labels: &[(&str, &str)]) -> Option<Vec<f64>> {
        let metric_key = self.create_metric_key(name, labels);
        let summaries = self.summaries.read().await;
        summaries.get(&metric_key).cloned()
    }

    /// Clear all metrics
    pub async fn clear_metrics(&self) {
        let mut counters = self.counters.write().await;
        counters.clear();
        let mut gauges = self.gauges.write().await;
        gauges.clear();
        let mut histograms = self.histograms.write().await;
        histograms.clear();
        let mut summaries = self.summaries.write().await;
        summaries.clear();
        let mut total_metrics = self.total_metrics.write().await;
        *total_metrics = 0;
        let mut last_metric_time = self.last_metric_time.write().await;
        *last_metric_time = None;
    }
}
