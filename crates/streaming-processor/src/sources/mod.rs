//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming data sources for the bridge
//! 
//! This module provides source implementations for streaming data
//! from various sources including Kafka, files, and HTTP endpoints.

pub mod kafka_source;
pub mod file_source;
pub mod http_source;

// Re-export source implementations
pub use kafka_source::KafkaSource;
pub use file_source::FileSource;
pub use http_source::HttpSource;

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::any::Any;

/// Source configuration trait
#[async_trait]
pub trait SourceConfig: Send + Sync {
    /// Get source name
    fn name(&self) -> &str;
    
    /// Get source version
    fn version(&self) -> &str;
    
    /// Validate configuration
    async fn validate(&self) -> BridgeResult<()>;
    
    /// Get configuration as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Source trait for streaming data sources
#[async_trait]
pub trait StreamSource: Send + Sync {
    /// Initialize the source
    async fn init(&mut self) -> BridgeResult<()>;
    
    /// Start consuming data
    async fn start(&mut self) -> BridgeResult<()>;
    
    /// Stop consuming data
    async fn stop(&mut self) -> BridgeResult<()>;
    
    /// Check if source is running
    fn is_running(&self) -> bool;
    
    /// Get source statistics
    async fn get_stats(&self) -> BridgeResult<SourceStats>;
    
    /// Get source name
    fn name(&self) -> &str;
    
    /// Get source version
    fn version(&self) -> &str;
}

/// Source statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStats {
    /// Source name
    pub source: String,
    
    /// Total records received
    pub total_records: u64,
    
    /// Records received in last minute
    pub records_per_minute: u64,
    
    /// Total bytes received
    pub total_bytes: u64,
    
    /// Bytes received in last minute
    pub bytes_per_minute: u64,
    
    /// Error count
    pub error_count: u64,
    
    /// Last record timestamp
    pub last_record_time: Option<DateTime<Utc>>,
    
    /// Connection status
    pub is_connected: bool,
    
    /// Lag (for Kafka sources)
    pub lag: Option<u64>,
}

/// Source factory for creating sources
pub struct SourceFactory;

impl SourceFactory {
    /// Create a source based on configuration
    pub async fn create_source(
        config: &dyn SourceConfig,
    ) -> BridgeResult<Box<dyn StreamSource>> {
        match config.name() {
            "kafka" => {
                let source = KafkaSource::new(config).await?;
                Ok(Box::new(source))
            }
            "file" => {
                let source = FileSource::new(config).await?;
                Ok(Box::new(source))
            }
            "http" => {
                let source = HttpSource::new(config).await?;
                Ok(Box::new(source))
            }
            _ => Err(bridge_core::BridgeError::configuration(
                format!("Unsupported source: {}", config.name())
            ))
        }
    }
}

/// Source manager for managing multiple sources
pub struct SourceManager {
    sources: HashMap<String, Box<dyn StreamSource>>,
    is_running: Arc<RwLock<bool>>,
}

impl SourceManager {
    /// Create new source manager
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Add source
    pub fn add_source(&mut self, name: String, source: Box<dyn StreamSource>) {
        self.sources.insert(name, source);
    }
    
    /// Remove source
    pub fn remove_source(&mut self, name: &str) -> Option<Box<dyn StreamSource>> {
        self.sources.remove(name)
    }
    
    /// Get source
    pub fn get_source(&self, name: &str) -> Option<&dyn StreamSource> {
        self.sources.get(name).map(|s| s.as_ref())
    }
    
    /// Get all source names
    pub fn get_source_names(&self) -> Vec<String> {
        self.sources.keys().cloned().collect()
    }
    
    /// Start all sources
    pub async fn start_all(&mut self) -> BridgeResult<()> {
        info!("Starting all sources");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        for (name, source) in &mut self.sources {
            info!("Starting source: {}", name);
            if let Err(e) = source.start().await {
                error!("Failed to start source {}: {}", name, e);
                return Err(e);
            }
        }
        
        info!("All sources started");
        Ok(())
    }
    
    /// Stop all sources
    pub async fn stop_all(&mut self) -> BridgeResult<()> {
        info!("Stopping all sources");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);
        
        for (name, source) in &mut self.sources {
            info!("Stopping source: {}", name);
            if let Err(e) = source.stop().await {
                error!("Failed to stop source {}: {}", name, e);
                return Err(e);
            }
        }
        
        info!("All sources stopped");
        Ok(())
    }
    
    /// Check if manager is running
    pub async fn is_running(&self) -> bool {
        let is_running = self.is_running.read().await;
        *is_running
    }
    
    /// Get source statistics
    pub async fn get_stats(&self) -> BridgeResult<HashMap<String, SourceStats>> {
        let mut stats = HashMap::new();
        
        for (name, source) in &self.sources {
            match source.get_stats().await {
                Ok(source_stats) => {
                    stats.insert(name.clone(), source_stats);
                }
                Err(e) => {
                    error!("Failed to get stats for source {}: {}", name, e);
                    return Err(e);
                }
            }
        }
        
        Ok(stats)
    }
}
