//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry receivers for OpenTelemetry data ingestion
//!
//! This module provides receiver implementations that use the protocol handlers
//! to ingest telemetry data from various sources.

pub mod http_receiver;
pub mod kafka_receiver;
pub mod otlp_receiver;
pub mod otlp_http_receiver;
pub mod otap_receiver;
pub mod simple_http_receiver;

// Re-export receiver implementations
pub use http_receiver::HttpReceiver;
pub use kafka_receiver::KafkaReceiver;
pub use otlp_receiver::OtlpReceiver;
pub use otlp_http_receiver::{OtlpHttpReceiver, OtlpHttpReceiverConfig};
pub use otap_receiver::OtapReceiver;
pub use simple_http_receiver::{SimpleHttpReceiver, SimpleHttpReceiverConfig};

use async_trait::async_trait;
use bridge_core::{
    traits::ReceiverStats, BridgeResult, TelemetryReceiver as BridgeTelemetryReceiver,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::protocols::{ProtocolConfig, ProtocolHandler};

/// Receiver configuration trait
#[async_trait]
pub trait ReceiverConfig: Send + Sync {
    /// Get receiver name
    fn name(&self) -> &str;

    /// Get receiver version
    fn version(&self) -> &str;

    /// Validate configuration
    async fn validate(&self) -> BridgeResult<()>;

    /// Get configuration as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Base receiver implementation
pub struct BaseReceiver {
    name: String,
    version: String,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ReceiverStats>>,
    protocol_handler: Option<Box<dyn ProtocolHandler>>,
}

impl BaseReceiver {
    /// Create new base receiver
    pub fn new(name: String, version: String) -> Self {
        let stats = ReceiverStats {
            total_records: 0,
            records_per_minute: 0,
            total_bytes: 0,
            bytes_per_minute: 0,
            error_count: 0,
            last_receive_time: None,
            protocol_stats: None,
        };

        Self {
            name,
            version,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
            protocol_handler: None,
        }
    }

    /// Set protocol handler
    pub fn set_protocol_handler(&mut self, handler: Box<dyn ProtocolHandler>) {
        self.protocol_handler = Some(handler);
    }

    /// Get protocol handler
    pub fn get_protocol_handler(&self) -> Option<&dyn ProtocolHandler> {
        self.protocol_handler.as_deref()
    }

    /// Update receiver statistics
    pub async fn update_stats(&self, records: usize, bytes: usize) {
        let mut stats = self.stats.write().await;
        stats.total_records += records as u64;
        stats.total_bytes += bytes as u64;
        stats.last_receive_time = Some(Utc::now());
    }

    /// Increment error count
    pub async fn increment_error_count(&self) {
        let mut stats = self.stats.write().await;
        stats.error_count += 1;
    }

    /// Record receive attempt
    pub async fn record_receive_attempt(&self) {
        // This could be used for tracking receive attempts
        // For now, just log it
        info!("Recording receive attempt for receiver: {}", self.name);
    }

    /// Record receive success
    pub async fn record_receive_success(&self, records: usize) {
        self.update_stats(records, 0).await;
    }

    /// Record receive error
    pub async fn record_receive_error(&self) {
        self.increment_error_count().await;
    }

    /// Get receiver statistics
    pub async fn get_stats(&self) -> BridgeResult<ReceiverStats> {
        let stats = self.stats.read().await;
        Ok(ReceiverStats {
            total_records: stats.total_records,
            records_per_minute: stats.records_per_minute,
            total_bytes: stats.total_bytes,
            bytes_per_minute: stats.bytes_per_minute,
            error_count: stats.error_count,
            last_receive_time: stats.last_receive_time,
            protocol_stats: None,
        })
    }
}

/// Receiver factory for creating receivers
pub struct ReceiverFactory;

impl ReceiverFactory {
    /// Create a receiver based on configuration
    pub async fn create_receiver(
        config: &dyn ReceiverConfig,
    ) -> BridgeResult<Box<dyn BridgeTelemetryReceiver>> {
        match config.name() {
            "otlp" => {
                let receiver = OtlpReceiver::new(config).await?;
                Ok(Box::new(receiver))
            }
            "otlp-http" => {
                let receiver = OtlpHttpReceiver::new(config).await?;
                Ok(Box::new(receiver))
            }
            "kafka" => {
                let receiver = KafkaReceiver::new(config).await?;
                Ok(Box::new(receiver))
            }
            "http" => {
                let receiver = HttpReceiver::new(config).await?;
                Ok(Box::new(receiver))
            }
            _ => Err(bridge_core::BridgeError::configuration(format!(
                "Unsupported receiver: {}",
                config.name()
            ))),
        }
    }
}

/// Receiver manager for managing multiple receivers
pub struct ReceiverManager {
    receivers: HashMap<String, Box<dyn BridgeTelemetryReceiver>>,
    is_running: Arc<RwLock<bool>>,
}

impl ReceiverManager {
    /// Create new receiver manager
    pub fn new() -> Self {
        Self {
            receivers: HashMap::new(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Add receiver
    pub fn add_receiver(&mut self, name: String, receiver: Box<dyn BridgeTelemetryReceiver>) {
        self.receivers.insert(name, receiver);
    }

    /// Remove receiver
    pub fn remove_receiver(&mut self, name: &str) -> Option<Box<dyn BridgeTelemetryReceiver>> {
        self.receivers.remove(name)
    }

    /// Get receiver
    pub fn get_receiver(&self, name: &str) -> Option<&dyn BridgeTelemetryReceiver> {
        self.receivers.get(name).map(|r| r.as_ref())
    }

    /// Get all receiver names
    pub fn get_receiver_names(&self) -> Vec<String> {
        self.receivers.keys().cloned().collect()
    }

    /// Start all receivers
    pub async fn start_all(&mut self) -> BridgeResult<()> {
        info!("Starting all receivers");

        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        for (name, receiver) in &mut self.receivers {
            info!("Starting receiver: {}", name);
            if let Err(e) = receiver.health_check().await {
                error!("Failed to start receiver {}: {}", name, e);
                return Err(e);
            }
        }

        info!("All receivers started");
        Ok(())
    }

    /// Stop all receivers
    pub async fn stop_all(&mut self) -> BridgeResult<()> {
        info!("Stopping all receivers");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        for (name, receiver) in &mut self.receivers {
            info!("Stopping receiver: {}", name);
            if let Err(e) = receiver.shutdown().await {
                error!("Failed to stop receiver {}: {}", name, e);
                return Err(e);
            }
        }

        info!("All receivers stopped");
        Ok(())
    }

    /// Check if manager is running
    pub async fn is_running(&self) -> bool {
        let is_running = self.is_running.read().await;
        *is_running
    }

    /// Get receiver statistics
    pub async fn get_stats(&self) -> BridgeResult<HashMap<String, ReceiverStats>> {
        let mut stats = HashMap::new();

        for (name, receiver) in &self.receivers {
            match receiver.get_stats().await {
                Ok(receiver_stats) => {
                    stats.insert(name.clone(), receiver_stats);
                }
                Err(e) => {
                    error!("Failed to get stats for receiver {}: {}", name, e);
                    return Err(e);
                }
            }
        }

        Ok(stats)
    }
}

/// Receiver health checker
pub struct ReceiverHealthChecker {
    receivers: Arc<RwLock<ReceiverManager>>,
}

impl ReceiverHealthChecker {
    /// Create new health checker
    pub fn new(receivers: Arc<RwLock<ReceiverManager>>) -> Self {
        Self { receivers }
    }

    /// Check health of all receivers
    pub async fn check_health(&self) -> BridgeResult<HashMap<String, bool>> {
        let receivers = self.receivers.read().await;
        let mut health_status = HashMap::new();

        for name in receivers.get_receiver_names() {
            if let Some(receiver) = receivers.get_receiver(&name) {
                match receiver.health_check().await {
                    Ok(healthy) => {
                        health_status.insert(name, healthy);
                    }
                    Err(e) => {
                        error!("Failed to check health for receiver {}: {}", name, e);
                        health_status.insert(name, false);
                    }
                }
            }
        }

        Ok(health_status)
    }
}
