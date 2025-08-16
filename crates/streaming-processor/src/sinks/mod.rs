//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming data sinks for the bridge
//!
//! This module provides sink implementations for streaming data
//! to various destinations including Kafka, files, and HTTP endpoints.

pub mod file_sink;
pub mod http_sink;
pub mod kafka_sink;

// Re-export sink implementations
pub use file_sink::FileSink;
pub use http_sink::HttpSink;
pub use kafka_sink::KafkaSink;

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// Sink configuration trait
#[async_trait]
pub trait SinkConfig: Send + Sync {
    /// Get sink name
    fn name(&self) -> &str;

    /// Get sink version
    fn version(&self) -> &str;

    /// Validate configuration
    async fn validate(&self) -> BridgeResult<()>;

    /// Get configuration as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Sink trait for streaming data sinks
#[async_trait]
pub trait StreamSink: Send + Sync {
    /// Initialize the sink
    async fn init(&mut self) -> BridgeResult<()>;

    /// Start the sink
    async fn start(&mut self) -> BridgeResult<()>;

    /// Stop the sink
    async fn stop(&mut self) -> BridgeResult<()>;

    /// Check if sink is running
    fn is_running(&self) -> bool;

    /// Send data to sink
    async fn send(&self, batch: TelemetryBatch) -> BridgeResult<()>;

    /// Get sink statistics
    async fn get_stats(&self) -> BridgeResult<SinkStats>;

    /// Get sink name
    fn name(&self) -> &str;

    /// Get sink version
    fn version(&self) -> &str;
}

/// Sink statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkStats {
    /// Sink name
    pub sink: String,

    /// Total batches sent
    pub total_batches: u64,

    /// Total records sent
    pub total_records: u64,

    /// Batches sent in last minute
    pub batches_per_minute: u64,

    /// Records sent in last minute
    pub records_per_minute: u64,

    /// Total bytes sent
    pub total_bytes: u64,

    /// Bytes sent in last minute
    pub bytes_per_minute: u64,

    /// Error count
    pub error_count: u64,

    /// Last send timestamp
    pub last_send_time: Option<DateTime<Utc>>,

    /// Connection status
    pub is_connected: bool,

    /// Latency in milliseconds
    pub latency_ms: u64,
}

/// Sink factory for creating sinks
pub struct SinkFactory;

impl SinkFactory {
    /// Create a sink based on configuration
    pub async fn create_sink(config: &dyn SinkConfig) -> BridgeResult<Box<dyn StreamSink>> {
        match config.name() {
            "kafka" => {
                let sink = KafkaSink::new(config).await?;
                Ok(Box::new(sink))
            }
            "file" => {
                let sink = FileSink::new(config).await?;
                Ok(Box::new(sink))
            }
            "http" => {
                let sink = HttpSink::new(config).await?;
                Ok(Box::new(sink))
            }
            _ => Err(bridge_core::BridgeError::configuration(format!(
                "Unsupported sink: {}",
                config.name()
            ))),
        }
    }
}

/// Sink manager for managing multiple sinks
pub struct SinkManager {
    sinks: HashMap<String, Box<dyn StreamSink>>,
    is_running: Arc<RwLock<bool>>,
}

impl SinkManager {
    /// Create new sink manager
    pub fn new() -> Self {
        Self {
            sinks: HashMap::new(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Add sink
    pub fn add_sink(&mut self, name: String, sink: Box<dyn StreamSink>) {
        self.sinks.insert(name, sink);
    }

    /// Remove sink
    pub fn remove_sink(&mut self, name: &str) -> Option<Box<dyn StreamSink>> {
        self.sinks.remove(name)
    }

    /// Get sink
    pub fn get_sink(&self, name: &str) -> Option<&dyn StreamSink> {
        self.sinks.get(name).map(|s| s.as_ref())
    }

    /// Get all sink names
    pub fn get_sink_names(&self) -> Vec<String> {
        self.sinks.keys().cloned().collect()
    }

    /// Start all sinks
    pub async fn start_all(&mut self) -> BridgeResult<()> {
        info!("Starting all sinks");

        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        for (name, sink) in &mut self.sinks {
            info!("Starting sink: {}", name);
            if let Err(e) = sink.start().await {
                error!("Failed to start sink {}: {}", name, e);
                return Err(e);
            }
        }

        info!("All sinks started");
        Ok(())
    }

    /// Stop all sinks
    pub async fn stop_all(&mut self) -> BridgeResult<()> {
        info!("Stopping all sinks");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        for (name, sink) in &mut self.sinks {
            info!("Stopping sink: {}", name);
            if let Err(e) = sink.stop().await {
                error!("Failed to stop sink {}: {}", name, e);
                return Err(e);
            }
        }

        info!("All sinks stopped");
        Ok(())
    }

    /// Check if manager is running
    pub async fn is_running(&self) -> bool {
        let is_running = self.is_running.read().await;
        *is_running
    }

    /// Send data to all sinks
    pub async fn send_to_all(&self, batch: TelemetryBatch) -> BridgeResult<Vec<BridgeResult<()>>> {
        let mut results = Vec::new();

        for (name, sink) in &self.sinks {
            match sink.send(batch.clone()).await {
                Ok(()) => {
                    results.push(Ok(()));
                    info!("Data sent to sink: {}", name);
                }
                Err(e) => {
                    error!("Failed to send data to sink {}: {}", name, e);
                    results.push(Err(e));
                }
            }
        }

        Ok(results)
    }

    /// Get sink statistics
    pub async fn get_stats(&self) -> BridgeResult<HashMap<String, SinkStats>> {
        let mut stats = HashMap::new();

        for (name, sink) in &self.sinks {
            match sink.get_stats().await {
                Ok(sink_stats) => {
                    stats.insert(name.clone(), sink_stats);
                }
                Err(e) => {
                    error!("Failed to get stats for sink {}: {}", name, e);
                    return Err(e);
                }
            }
        }

        Ok(stats)
    }
}
