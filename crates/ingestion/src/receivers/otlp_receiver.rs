//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OTLP receiver implementation
//!
//! This module provides an OTLP receiver that can handle both OTLP Arrow
//! and OTLP gRPC protocols.

use async_trait::async_trait;
use bridge_core::{
    traits::ReceiverStats, BridgeResult, TelemetryBatch,
    TelemetryReceiver as BridgeTelemetryReceiver,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use uuid::Uuid;

use super::{BaseReceiver, ReceiverConfig};
use crate::protocols::otlp_arrow::{OtlpArrowConfig, OtlpArrowProtocol};
use crate::protocols::otlp_grpc::{OtlpGrpcConfig, OtlpGrpcProtocol};
use crate::protocols::{ProtocolConfig, ProtocolHandler};

/// OTLP receiver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpReceiverConfig {
    /// Receiver name
    pub name: String,

    /// Receiver version
    pub version: String,

    /// Protocol type (otlp-arrow or otlp-grpc)
    pub protocol: OtlpProtocolType,

    /// Protocol configuration
    pub protocol_config: OtlpProtocolConfig,

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

/// OTLP protocol types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OtlpProtocolType {
    Arrow,
    Grpc,
}

/// OTLP protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OtlpProtocolConfig {
    Arrow(OtlpArrowConfig),
    Grpc(OtlpGrpcConfig),
}

impl OtlpReceiverConfig {
    /// Create new OTLP Arrow receiver configuration
    pub fn new_arrow(endpoint: String, port: u16) -> Self {
        Self {
            name: "otlp".to_string(),
            version: "1.0.0".to_string(),
            protocol: OtlpProtocolType::Arrow,
            protocol_config: OtlpProtocolConfig::Arrow(OtlpArrowConfig::new(endpoint, port)),
            batch_size: 1000,
            buffer_size: 10000,
            enable_compression: true,
            auth_token: None,
            headers: HashMap::new(),
            timeout_secs: 30,
        }
    }

    /// Create new OTLP gRPC receiver configuration
    pub fn new_grpc(endpoint: String, port: u16) -> Self {
        Self {
            name: "otlp".to_string(),
            version: "1.0.0".to_string(),
            protocol: OtlpProtocolType::Grpc,
            protocol_config: OtlpProtocolConfig::Grpc(OtlpGrpcConfig::new(endpoint, port)),
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
impl ReceiverConfig for OtlpReceiverConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.batch_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "OTLP receiver batch size cannot be 0",
            ));
        }

        if self.buffer_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "OTLP receiver buffer size cannot be 0",
            ));
        }

        // Validate protocol configuration
        match &self.protocol_config {
            OtlpProtocolConfig::Arrow(config) => config.validate().await?,
            OtlpProtocolConfig::Grpc(config) => config.validate().await?,
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// OTLP receiver implementation
pub struct OtlpReceiver {
    base: BaseReceiver,
    config: OtlpReceiverConfig,
    protocol_handler: Option<Arc<tokio::sync::Mutex<Box<dyn ProtocolHandler>>>>,
}

impl OtlpReceiver {
    /// Create new OTLP receiver
    pub async fn new(config: &dyn ReceiverConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<OtlpReceiverConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration("Invalid OTLP receiver configuration")
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
        info!("Initializing OTLP protocol handler");

        let protocol_handler: Box<dyn ProtocolHandler> = match &self.config.protocol_config {
            OtlpProtocolConfig::Arrow(config) => {
                let handler = OtlpArrowProtocol::new(config).await?;
                Box::new(handler)
            }
            OtlpProtocolConfig::Grpc(config) => {
                let handler = OtlpGrpcProtocol::new(config).await?;
                Box::new(handler)
            }
        };

        self.protocol_handler = Some(Arc::new(Mutex::new(protocol_handler)));

        info!("OTLP protocol handler initialized");
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

    /// Receive telemetry data
    async fn receive_data(&self) -> BridgeResult<TelemetryBatch> {
        if let Some(handler) = &self.protocol_handler {
            let handler = handler.lock().await;

            // Try to receive data from the protocol handler
            match handler.receive_data().await {
                Ok(Some(batch)) => {
                    info!("Received OTLP batch with {} records", batch.records.len());
                    Ok(batch)
                }
                Ok(None) => {
                    // No data available, return empty batch
                    Ok(TelemetryBatch {
                        id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        source: "otlp".to_string(),
                        size: 0,
                        records: vec![],
                        metadata: HashMap::new(),
                    })
                }
                Err(e) => {
                    info!("Error receiving OTLP data: {}", e);
                    Err(e)
                }
            }
        } else {
            Err(bridge_core::BridgeError::configuration(
                "OTLP protocol handler not initialized".to_string(),
            ))
        }
    }

    /// Process incoming OTLP data
    async fn process_otlp_data(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        use crate::conversion::{OtlpConverter, TelemetryConverter};

        let converter = OtlpConverter::new();

        // Try to convert OTLP data to internal format
        match converter.convert(data, "otlp", "internal").await {
            Ok(internal_data) => {
                // Deserialize the internal format
                let batch: TelemetryBatch =
                    serde_json::from_slice(&internal_data).map_err(|e| {
                        bridge_core::BridgeError::serialization(format!(
                            "Failed to deserialize internal format: {}",
                            e
                        ))
                    })?;

                Ok(batch)
            }
            Err(e) => {
                info!("Failed to convert OTLP data: {}", e);
                Err(e)
            }
        }
    }

    /// Validate and sanitize received data
    async fn validate_data(&self, mut batch: TelemetryBatch) -> BridgeResult<TelemetryBatch> {
        use crate::conversion::DataValidator;

        let validator = DataValidator::new();
        validator.validate_batch(&mut batch)?;

        Ok(batch)
    }
}

#[async_trait]
impl BridgeTelemetryReceiver for OtlpReceiver {
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
        info!("Shutting down OTLP receiver");

        // Stop protocol handler
        if let Some(handler) = &self.protocol_handler {
            let mut handler = handler.lock().await;
            handler.stop().await?;
        }

        info!("OTLP receiver shutdown completed");
        Ok(())
    }
}

impl OtlpReceiver {
    /// Initialize the receiver
    pub async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing OTLP receiver");

        // Initialize protocol handler
        self.init_protocol_handler().await?;

        // Start protocol handler
        self.start_protocol_handler().await?;

        info!("OTLP receiver initialized");
        Ok(())
    }

    /// Get protocol type
    pub fn get_protocol_type(&self) -> &OtlpProtocolType {
        &self.config.protocol
    }

    /// Get protocol configuration
    pub fn get_protocol_config(&self) -> &OtlpProtocolConfig {
        &self.config.protocol_config
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
