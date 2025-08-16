//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Kafka receiver implementation
//!
//! This module provides a Kafka receiver that uses the Kafka protocol handler
//! to ingest telemetry data from Kafka topics.

use async_trait::async_trait;
use bridge_core::{
    traits::ReceiverStats, BridgeResult, TelemetryBatch,
    TelemetryReceiver as BridgeTelemetryReceiver,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use super::{BaseReceiver, ReceiverConfig};
#[cfg(feature = "kafka")]
use crate::protocols::kafka::{KafkaConfig, KafkaProtocol};
use crate::protocols::ProtocolHandler;

/// Kafka receiver configuration
#[cfg_attr(feature = "kafka", derive(Debug, Clone, Serialize, Deserialize))]
#[cfg_attr(not(feature = "kafka"), derive(Debug, Clone))]
pub struct KafkaReceiverConfig {
    /// Receiver name
    pub name: String,

    /// Receiver version
    pub version: String,

    /// Kafka configuration
    #[cfg(feature = "kafka")]
    pub kafka_config: KafkaConfig,
    #[cfg(not(feature = "kafka"))]
    pub kafka_config: (),

    /// Batch size for processing
    pub batch_size: usize,

    /// Buffer size for incoming data
    pub buffer_size: usize,

    /// Enable compression
    pub enable_compression: bool,

    /// Authentication token (optional)
    pub auth_token: Option<String>,

    /// Additional headers
    pub headers: HashMap<String, String>,

    /// Timeout in seconds
    pub timeout_secs: u64,
}

impl KafkaReceiverConfig {
    /// Create new Kafka receiver configuration
    pub fn new(bootstrap_servers: Vec<String>, topic: String, group_id: String) -> Self {
        Self {
            name: "kafka".to_string(),
            version: "1.0.0".to_string(),
            #[cfg(feature = "kafka")]
            kafka_config: KafkaConfig::new(bootstrap_servers, topic, group_id),
            #[cfg(not(feature = "kafka"))]
            kafka_config: (),
            batch_size: 1000,
            buffer_size: 10000,
            enable_compression: true,
            auth_token: None,
            headers: HashMap::new(),
            timeout_secs: 30,
        }
    }
}

#[async_trait]
impl ReceiverConfig for KafkaReceiverConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.batch_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Kafka receiver batch size cannot be 0",
            ));
        }

        if self.buffer_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Kafka receiver buffer size cannot be 0",
            ));
        }

        // Validate Kafka configuration
        #[cfg(feature = "kafka")]
        self.kafka_config.validate().await?;
        #[cfg(not(feature = "kafka"))]
        // No validation needed when Kafka is disabled
        ();

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Kafka receiver implementation
#[cfg(feature = "kafka")]
pub struct KafkaReceiver {
    base: BaseReceiver,
    config: KafkaReceiverConfig,
    protocol_handler: Option<Arc<tokio::sync::Mutex<Box<dyn ProtocolHandler>>>>,
}

#[cfg(not(feature = "kafka"))]
pub struct KafkaReceiver {
    base: BaseReceiver,
    config: KafkaReceiverConfig,
    protocol_handler: Option<Arc<tokio::sync::Mutex<Box<dyn ProtocolHandler>>>>,
}

impl KafkaReceiver {
    /// Create new Kafka receiver
    pub async fn new(config: &dyn ReceiverConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<KafkaReceiverConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration("Invalid Kafka receiver configuration")
            })?
            .clone();

        config.validate().await?;

        let base = BaseReceiver::new(config.name.clone(), config.version.clone());

        Ok(Self {
            base,
            config,
            protocol_handler: None,
        })
    }

    /// Initialize protocol handler
    async fn init_protocol_handler(&mut self) -> BridgeResult<()> {
        info!("Initializing Kafka protocol handler");

        #[cfg(feature = "kafka")]
        {
            let protocol_handler = KafkaProtocol::new(&self.config.kafka_config).await?;
            self.protocol_handler = Some(Arc::new(Mutex::new(Box::new(protocol_handler))));
        }
        #[cfg(not(feature = "kafka"))]
        {
            // Create a placeholder handler when Kafka is disabled
            self.protocol_handler = None;
        }

        info!("Kafka protocol handler initialized");
        Ok(())
    }

    /// Start protocol handler
    async fn start_protocol_handler(&mut self) -> BridgeResult<()> {
        if let Some(handler) = &self.protocol_handler {
            let mut handler = handler.lock().await;
            handler.start().await?;
        }
        Ok(())
    }

    /// Stop protocol handler
    async fn stop_protocol_handler(&mut self) -> BridgeResult<()> {
        if let Some(handler) = &self.protocol_handler {
            let mut handler = handler.lock().await;
            handler.stop().await?;
        }
        Ok(())
    }

    /// Receive telemetry data from Kafka
    async fn receive_data(&self) -> BridgeResult<TelemetryBatch> {
        // Get the protocol handler and receive data from Kafka
        if let Some(handler) = &self.protocol_handler {
            let handler = handler.lock().await;

            // Try to receive data from the protocol handler
            match handler.receive_data().await {
                Ok(Some(batch)) => {
                    // Data was available, return it
                    info!(
                        "Received telemetry batch from Kafka with {} records",
                        batch.size
                    );
                    Ok(batch)
                }
                Ok(None) => {
                    // No data available at the moment, return an empty batch
                    info!("No telemetry data available from Kafka at this time");
                    Ok(TelemetryBatch {
                        id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        source: "kafka".to_string(),
                        size: 0,
                        records: Vec::new(),
                        metadata: HashMap::new(),
                    })
                }
                Err(e) => {
                    // Error occurred while receiving data
                    error!("Failed to receive data from Kafka protocol handler: {}", e);
                    Err(e)
                }
            }
        } else {
            // Protocol handler not initialized, return an error
            Err(bridge_core::BridgeError::configuration(
                "Kafka protocol handler not initialized".to_string(),
            ))
        }
    }
}

#[async_trait]
impl BridgeTelemetryReceiver for KafkaReceiver {
    async fn receive(&self) -> BridgeResult<TelemetryBatch> {
        match self.receive_data().await {
            Ok(batch) => {
                // Update statistics
                self.base.update_stats(batch.size, 0).await;
                Ok(batch)
            }
            Err(e) => {
                // Increment error count
                self.base.increment_error_count().await;
                Err(e)
            }
        }
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        if let Some(handler) = &self.protocol_handler {
            let handler = handler.lock().await;
            match handler.get_stats().await {
                Ok(stats) => Ok(stats.is_connected),
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    async fn get_stats(&self) -> BridgeResult<ReceiverStats> {
        let stats = self.base.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down Kafka receiver");

        // Stop protocol handler
        if let Some(handler) = &self.protocol_handler {
            let mut handler = handler.lock().await;
            handler.stop().await?;
        }

        info!("Kafka receiver shutdown completed");
        Ok(())
    }
}

impl KafkaReceiver {
    /// Initialize the receiver
    pub async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing Kafka receiver");

        // Initialize protocol handler
        self.init_protocol_handler().await?;

        // Start protocol handler
        self.start_protocol_handler().await?;

        info!("Kafka receiver initialized");
        Ok(())
    }

    /// Get Kafka configuration
    #[cfg(feature = "kafka")]
    pub fn get_kafka_config(&self) -> &KafkaConfig {
        &self.config.kafka_config
    }

    #[cfg(not(feature = "kafka"))]
    pub fn get_kafka_config(&self) -> &() {
        &self.config.kafka_config
    }

    /// Check if receiver is running
    pub async fn is_running(&self) -> bool {
        if let Some(handler) = &self.protocol_handler {
            let handler = handler.lock().await;
            handler.is_running()
        } else {
            false
        }
    }
}
