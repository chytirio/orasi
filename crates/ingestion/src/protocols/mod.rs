//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Protocol implementations for OpenTelemetry data ingestion
//!
//! This module provides protocol-specific implementations for ingesting
//! OpenTelemetry data from various sources including OTLP Arrow, Kafka,
//! and OTLP gRPC.

pub mod kafka;
pub mod otlp_arrow;
pub mod otlp_grpc;
pub mod otap;

// Re-export protocol implementations
pub use kafka::KafkaProtocol;
pub use otlp_arrow::OtlpArrowProtocol;
pub use otlp_grpc::OtlpGrpcProtocol;
pub use otap::OtapProtocol;

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;

/// Protocol configuration trait
#[async_trait]
pub trait ProtocolConfig: Send + Sync {
    /// Get protocol name
    fn name(&self) -> &str;

    /// Get protocol version
    fn version(&self) -> &str;

    /// Validate configuration
    async fn validate(&self) -> BridgeResult<()>;

    /// Get configuration as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Protocol handler trait
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    /// Initialize the protocol handler
    async fn init(&mut self) -> BridgeResult<()>;

    /// Start listening for data
    async fn start(&mut self) -> BridgeResult<()>;

    /// Stop listening for data
    async fn stop(&mut self) -> BridgeResult<()>;

    /// Check if handler is running
    fn is_running(&self) -> bool;

    /// Get handler statistics
    async fn get_stats(&self) -> BridgeResult<ProtocolStats>;

    /// Receive data from the protocol handler
    /// Returns None if no data is available (non-blocking)
    async fn receive_data(&self) -> BridgeResult<Option<TelemetryBatch>>;

    /// Get handler as Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Protocol statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolStats {
    /// Protocol name
    pub protocol: String,

    /// Total messages received
    pub total_messages: u64,

    /// Messages received in last minute
    pub messages_per_minute: u64,

    /// Total bytes received
    pub total_bytes: u64,

    /// Bytes received in last minute
    pub bytes_per_minute: u64,

    /// Error count
    pub error_count: u64,

    /// Last message timestamp
    pub last_message_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Connection status
    pub is_connected: bool,
}

/// Protocol factory for creating protocol handlers
pub struct ProtocolFactory;

impl ProtocolFactory {
    /// Create a protocol handler based on configuration
    pub async fn create_handler(
        config: &dyn ProtocolConfig,
    ) -> BridgeResult<Box<dyn ProtocolHandler>> {
        match config.name() {
            "otlp-arrow" => {
                let handler = OtlpArrowProtocol::new(config).await?;
                Ok(Box::new(handler))
            }
            "otap" => {
                let handler = OtapProtocol::new(config).await?;
                Ok(Box::new(handler))
            }
            "kafka" => {
                let handler = KafkaProtocol::new(config).await?;
                Ok(Box::new(handler))
            }
            "otlp-grpc" => {
                let handler = OtlpGrpcProtocol::new(config).await?;
                Ok(Box::new(handler))
            }
            _ => Err(bridge_core::BridgeError::configuration(format!(
                "Unsupported protocol: {}",
                config.name()
            ))),
        }
    }
}

/// Message payload types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    /// Arrow format data
    Arrow(Vec<u8>),
    /// JSON format data
    Json(Vec<u8>),
    /// Protobuf format data
    Protobuf(Vec<u8>),
}

/// Protocol message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    /// Message ID
    pub id: String,

    /// Message timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Protocol name
    pub protocol: String,

    /// Message payload
    pub payload: MessagePayload,

    /// Message metadata
    pub metadata: HashMap<String, String>,
}

/// Protocol message handler trait
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle incoming protocol message
    async fn handle_message(&self, message: ProtocolMessage) -> BridgeResult<TelemetryBatch>;

    /// Handle batch of messages
    async fn handle_batch(
        &self,
        messages: Vec<ProtocolMessage>,
    ) -> BridgeResult<Vec<TelemetryBatch>>;
}
