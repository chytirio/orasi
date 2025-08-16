//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming data metrics for the bridge
//!
//! This module provides metrics collection functionality for streaming data
//! including metrics collectors and factories.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// Stream metrics for tracking streaming data processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMetrics {
    /// Metrics ID
    pub id: String,

    /// Total records processed
    pub total_records: u64,

    /// Total batches processed
    pub total_batches: u64,

    /// Records processed per second
    pub records_per_second: f64,

    /// Batches processed per second
    pub batches_per_second: f64,

    /// Total processing time in milliseconds
    pub total_processing_time_ms: u64,

    /// Average processing time per batch in milliseconds
    pub avg_processing_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Success count
    pub success_count: u64,

    /// Last update timestamp
    pub last_update: DateTime<Utc>,

    /// Component-specific metrics
    pub component_metrics: HashMap<String, ComponentMetrics>,
}

/// Component-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetrics {
    /// Component name
    pub component_name: String,

    /// Component type
    pub component_type: String,

    /// Records processed by component
    pub records_processed: u64,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,

    /// Error count
    pub error_count: u64,

    /// Success count
    pub success_count: u64,

    /// Last update timestamp
    pub last_update: DateTime<Utc>,
}

/// Metrics collector trait for collecting streaming metrics
#[async_trait]
pub trait MetricsCollector: Send + Sync {
    /// Get collector name
    fn name(&self) -> &str;

    /// Get collector version
    fn version(&self) -> &str;

    /// Initialize the collector
    async fn init(&mut self) -> BridgeResult<()>;

    /// Start collecting metrics
    async fn start(&mut self) -> BridgeResult<()>;

    /// Stop collecting metrics
    async fn stop(&mut self) -> BridgeResult<()>;

    /// Check if collector is running
    fn is_running(&self) -> bool;

    /// Record metrics
    async fn record_metrics(&self, metrics: StreamMetrics) -> BridgeResult<()>;

    /// Get collected metrics
    async fn get_metrics(&self) -> BridgeResult<Vec<StreamMetrics>>;

    /// Get collector statistics
    async fn get_stats(&self) -> BridgeResult<CollectorStats>;
}

/// Collector statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorStats {
    /// Collector name
    pub collector_name: String,

    /// Total metrics collected
    pub total_metrics: u64,

    /// Metrics collected in last minute
    pub metrics_per_minute: u64,

    /// Total collection time in milliseconds
    pub total_collection_time_ms: u64,

    /// Average collection time per metric in milliseconds
    pub avg_collection_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last collection timestamp
    pub last_collection_time: Option<DateTime<Utc>>,

    /// Collection status
    pub is_collecting: bool,
}

/// In-memory metrics collector implementation
pub struct InMemoryMetricsCollector {
    name: String,
    version: String,
    is_running: Arc<RwLock<bool>>,
    metrics: Arc<RwLock<Vec<StreamMetrics>>>,
    stats: Arc<RwLock<CollectorStats>>,
}

impl InMemoryMetricsCollector {
    /// Create new in-memory metrics collector
    pub fn new(name: String) -> Self {
        let stats = CollectorStats {
            collector_name: name.clone(),
            total_metrics: 0,
            metrics_per_minute: 0,
            total_collection_time_ms: 0,
            avg_collection_time_ms: 0.0,
            error_count: 0,
            last_collection_time: None,
            is_collecting: false,
        };

        Self {
            name,
            version: "1.0.0".to_string(),
            is_running: Arc::new(RwLock::new(false)),
            metrics: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(stats)),
        }
    }
}

#[async_trait]
impl MetricsCollector for InMemoryMetricsCollector {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing in-memory metrics collector: {}", self.name);
        Ok(())
    }

    async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting in-memory metrics collector: {}", self.name);

        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_collecting = true;
        }

        Ok(())
    }

    async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping in-memory metrics collector: {}", self.name);

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_collecting = false;
        }

        Ok(())
    }

    fn is_running(&self) -> bool {
        // This is a simplified check - in practice we'd need to handle the async nature
        false
    }

    async fn record_metrics(&self, metrics: StreamMetrics) -> BridgeResult<()> {
        let start_time = std::time::Instant::now();

        let mut metrics_list = self.metrics.write().await;
        metrics_list.push(metrics);

        let collection_time = start_time.elapsed().as_millis() as u64;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_metrics += 1;
            stats.total_collection_time_ms += collection_time;
            stats.avg_collection_time_ms =
                stats.total_collection_time_ms as f64 / stats.total_metrics as f64;
            stats.last_collection_time = Some(Utc::now());
        }

        Ok(())
    }

    async fn get_metrics(&self) -> BridgeResult<Vec<StreamMetrics>> {
        let metrics = self.metrics.read().await;
        Ok(metrics.clone())
    }

    async fn get_stats(&self) -> BridgeResult<CollectorStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}

/// Metrics manager for managing multiple metrics collectors
pub struct MetricsManager {
    collectors: HashMap<String, Box<dyn MetricsCollector>>,
}

impl MetricsManager {
    /// Create new metrics manager
    pub fn new() -> Self {
        Self {
            collectors: HashMap::new(),
        }
    }

    /// Add metrics collector
    pub fn add_collector(&mut self, name: String, collector: Box<dyn MetricsCollector>) {
        self.collectors.insert(name, collector);
    }

    /// Remove metrics collector
    pub fn remove_collector(&mut self, name: &str) -> Option<Box<dyn MetricsCollector>> {
        self.collectors.remove(name)
    }

    /// Get metrics collector
    pub fn get_collector(&self, name: &str) -> Option<&dyn MetricsCollector> {
        self.collectors.get(name).map(|c| c.as_ref())
    }

    /// Get all collector names
    pub fn get_collector_names(&self) -> Vec<String> {
        self.collectors.keys().cloned().collect()
    }

    /// Start all collectors
    pub async fn start_all(&mut self) -> BridgeResult<()> {
        info!("Starting all metrics collectors");

        for (name, collector) in &mut self.collectors {
            info!("Starting metrics collector: {}", name);
            if let Err(e) = collector.start().await {
                error!("Failed to start metrics collector {}: {}", name, e);
                return Err(e);
            }
        }

        info!("All metrics collectors started");
        Ok(())
    }

    /// Stop all collectors
    pub async fn stop_all(&mut self) -> BridgeResult<()> {
        info!("Stopping all metrics collectors");

        for (name, collector) in &mut self.collectors {
            info!("Stopping metrics collector: {}", name);
            if let Err(e) = collector.stop().await {
                error!("Failed to stop metrics collector {}: {}", name, e);
                return Err(e);
            }
        }

        info!("All metrics collectors stopped");
        Ok(())
    }

    /// Record metrics to all collectors
    pub async fn record_metrics(
        &self,
        metrics: StreamMetrics,
    ) -> BridgeResult<Vec<BridgeResult<()>>> {
        let mut results = Vec::new();

        for (name, collector) in &self.collectors {
            match collector.record_metrics(metrics.clone()).await {
                Ok(()) => {
                    results.push(Ok(()));
                    info!("Metrics recorded to collector: {}", name);
                }
                Err(e) => {
                    error!("Failed to record metrics to collector {}: {}", name, e);
                    results.push(Err(e));
                }
            }
        }

        Ok(results)
    }

    /// Get metrics from all collectors
    pub async fn get_all_metrics(&self) -> BridgeResult<HashMap<String, Vec<StreamMetrics>>> {
        let mut all_metrics = HashMap::new();

        for (name, collector) in &self.collectors {
            match collector.get_metrics().await {
                Ok(metrics) => {
                    all_metrics.insert(name.clone(), metrics);
                }
                Err(e) => {
                    error!("Failed to get metrics from collector {}: {}", name, e);
                    return Err(e);
                }
            }
        }

        Ok(all_metrics)
    }
}

/// Metrics factory for creating metrics collectors
pub struct MetricsFactory;

impl MetricsFactory {
    /// Create in-memory metrics collector
    pub fn create_in_memory_collector(name: String) -> Box<dyn MetricsCollector> {
        Box::new(InMemoryMetricsCollector::new(name))
    }

    /// Create metrics collector by type
    pub fn create_collector(
        collector_type: &str,
        name: String,
    ) -> Option<Box<dyn MetricsCollector>> {
        match collector_type {
            "memory" => Some(Self::create_in_memory_collector(name)),
            _ => None,
        }
    }

    /// Get available collector types
    pub fn get_available_collector_types() -> Vec<String> {
        vec!["memory".to_string()]
    }
}
