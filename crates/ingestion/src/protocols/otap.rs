//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OTel-Arrow Protocol (OTAP) implementation
//!
//! This module provides support for ingesting OpenTelemetry data using the
//! OTel-Arrow Protocol (OTAP) which provides significant compression improvements
//! over standard OTLP.

use async_trait::async_trait;
use bridge_core::{
    types::{LogData, MetricData, MetricValue, TelemetryData, TelemetryRecord, TelemetryType},
    BridgeResult, TelemetryBatch,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, mpsc::error::TryRecvError, Mutex};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{MessageHandler, ProtocolConfig, ProtocolHandler, ProtocolMessage, ProtocolStats};

/// OTAP protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtapConfig {
    /// Protocol name
    pub name: String,

    /// Protocol version
    pub version: String,

    /// gRPC endpoint address
    pub endpoint: String,

    /// Port to listen on
    pub port: u16,

    /// Enable TLS
    pub enable_tls: bool,

    /// TLS certificate path
    pub tls_cert_path: Option<String>,

    /// TLS key path
    pub tls_key_path: Option<String>,

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

    /// Max concurrent requests
    pub max_concurrent_requests: usize,

    /// Enable fallback to OTLP
    pub enable_otlp_fallback: bool,

    /// Schema reset interval (in batches)
    pub schema_reset_interval: usize,
}

impl OtapConfig {
    /// Create new OTAP configuration
    pub fn new(endpoint: String, port: u16) -> Self {
        Self {
            name: "otap".to_string(),
            version: "1.0.0".to_string(),
            endpoint,
            port,
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            batch_size: 1000,
            buffer_size: 10000,
            enable_compression: true,
            auth_token: None,
            headers: HashMap::new(),
            timeout_secs: 30,
            max_concurrent_requests: 100,
            enable_otlp_fallback: true,
            schema_reset_interval: 1000,
        }
    }
}

#[async_trait]
impl ProtocolConfig for OtapConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.endpoint.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "OTAP endpoint cannot be empty",
            ));
        }

        if self.port == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "OTAP port cannot be 0",
            ));
        }

        if self.batch_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "OTAP batch size cannot be 0",
            ));
        }

        if self.enable_tls {
            if self.tls_cert_path.is_none() || self.tls_key_path.is_none() {
                return Err(bridge_core::BridgeError::configuration(
                    "TLS certificate and key paths are required when TLS is enabled",
                ));
            }
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// OTAP protocol handler
pub struct OtapProtocol {
    config: OtapConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ProtocolStats>>,
    message_handler: Arc<dyn MessageHandler>,
    server: Option<OtapServer>,
    batch_count: Arc<RwLock<usize>>,
    data_tx: Option<mpsc::UnboundedSender<TelemetryBatch>>,
    data_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<TelemetryBatch>>>>,
}

/// OTAP server wrapper
struct OtapServer {
    // TODO: Implement actual gRPC server with OTAP services
}

impl OtapProtocol {
    /// Create new OTAP protocol handler
    pub async fn new(config: &dyn ProtocolConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<OtapConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration("Invalid OTAP configuration")
            })?
            .clone();

        config.validate().await?;

        let stats = ProtocolStats {
            protocol: config.name.clone(),
            total_messages: 0,
            messages_per_minute: 0,
            total_bytes: 0,
            bytes_per_minute: 0,
            error_count: 0,
            last_message_time: None,
            is_connected: false,
        };

        // Create data channel for communication with receiver
        let (data_tx, data_rx) = mpsc::unbounded_channel::<TelemetryBatch>();

        Ok(Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
            message_handler: Arc::new(OtapMessageHandler::new()),
            server: None,
            batch_count: Arc::new(RwLock::new(0)),
            data_tx: Some(data_tx),
            data_rx: Arc::new(Mutex::new(Some(data_rx))),
        })
    }

    /// Initialize gRPC server
    async fn init_server(&mut self) -> BridgeResult<()> {
        info!(
            "Initializing OTAP gRPC server on {}:{}",
            self.config.endpoint, self.config.port
        );

        // TODO: Implement gRPC server initialization with OTAP services
        // This would use tonic to create the server with the OTAP services

        self.server = Some(OtapServer {});

        info!("OTAP gRPC server initialized");
        Ok(())
    }

    /// Start gRPC server
    async fn start_server(&self) -> BridgeResult<()> {
        info!("Starting OTAP gRPC server");

        // TODO: Implement gRPC server startup
        // This would start the tonic server with OTAP services

        Ok(())
    }

    /// Process OTAP Arrow data
    async fn process_otap_data(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        // Use the otel-arrow-rust crate to decode OTAP data
        // This is a placeholder implementation that will be replaced with actual OTAP decoding
        
        info!("Processing OTAP Arrow data of {} bytes", data.len());

        // TODO: Implement actual OTAP decoding using otel-arrow-rust
        // This would decode the Arrow IPC format and convert to TelemetryBatch
        
        // For now, create a placeholder batch
        let records = vec![
            TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name: "otap_metric_1".to_string(),
                    description: Some("OTAP-encoded metric".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: bridge_core::types::MetricType::Counter,
                    value: MetricValue::Counter(42.0),
                    labels: HashMap::from([
                        ("source".to_string(), "otap".to_string()),
                        ("data_size".to_string(), data.len().to_string()),
                    ]),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::from([
                    ("protocol".to_string(), "otap".to_string()),
                    ("format".to_string(), "arrow".to_string()),
                ]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            },
        ];

        // Update batch count for schema reset tracking
        {
            let mut batch_count = self.batch_count.write().await;
            *batch_count += 1;
        }

        let batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "otap".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::from([
                ("protocol".to_string(), "otap".to_string()),
                ("content_type".to_string(), "application/arrow".to_string()),
                ("compression".to_string(), self.config.enable_compression.to_string()),
            ]),
        };

        // Send the batch to the receiver through the data channel
        if let Some(data_tx) = &self.data_tx {
            if let Err(e) = data_tx.send(batch.clone()) {
                error!("Failed to send OTAP data to receiver: {}", e);
            }
        }

        Ok(batch)
    }

    /// Send data to receiver through the data channel
    async fn send_data_to_receiver(&self, batch: TelemetryBatch) -> BridgeResult<()> {
        if let Some(data_tx) = &self.data_tx {
            data_tx.send(batch)
                .map_err(|e| bridge_core::BridgeError::internal(format!("Failed to send data to receiver: {}", e)))?;
        }
        Ok(())
    }

    /// Simulate incoming OTAP data for testing purposes
    /// This method can be called to simulate data reception from the OTAP protocol
    pub async fn simulate_incoming_data(&self) -> BridgeResult<()> {
        info!("Simulating incoming OTAP data");

        // Create a simulated OTAP batch
        let records = vec![
            TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name: "simulated_otap_metric".to_string(),
                    description: Some("Simulated OTAP metric data".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: bridge_core::types::MetricType::Counter,
                    value: MetricValue::Counter(123.45),
                    labels: HashMap::from([
                        ("source".to_string(), "otap_simulator".to_string()),
                        ("protocol".to_string(), "otap".to_string()),
                        ("simulated".to_string(), "true".to_string()),
                    ]),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::from([
                    ("protocol".to_string(), "otap".to_string()),
                    ("simulator".to_string(), "true".to_string()),
                    ("data_type".to_string(), "simulated".to_string()),
                ]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            },
            TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Log,
                data: TelemetryData::Log(LogData {
                    message: "Simulated OTAP log message".to_string(),
                    level: bridge_core::types::LogLevel::Info,
                    timestamp: Utc::now(),
                    attributes: HashMap::from([
                        ("source".to_string(), "otap_simulator".to_string()),
                        ("protocol".to_string(), "otap".to_string()),
                    ]),
                    body: Some("Simulated OTAP log body".to_string()),
                    severity_number: Some(9), // INFO level
                    severity_text: Some("INFO".to_string()),
                }),
                attributes: HashMap::from([
                    ("protocol".to_string(), "otap".to_string()),
                    ("simulator".to_string(), "true".to_string()),
                ]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            },
        ];

        let batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "otap-simulator".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::from([
                ("protocol".to_string(), "otap".to_string()),
                ("content_type".to_string(), "application/arrow".to_string()),
                ("compression".to_string(), self.config.enable_compression.to_string()),
                ("simulated".to_string(), "true".to_string()),
            ]),
        };

        // Send the simulated data to the receiver
        self.send_data_to_receiver(batch).await?;

        info!("Simulated OTAP data sent to receiver");
        Ok(())
    }

    /// Check if schema should be reset
    async fn should_reset_schema(&self) -> bool {
        let batch_count = self.batch_count.read().await;
        *batch_count >= self.config.schema_reset_interval
    }

    /// Reset schema and batch count
    async fn reset_schema(&self) -> BridgeResult<()> {
        info!("Resetting OTAP schema");
        
        let mut batch_count = self.batch_count.write().await;
        *batch_count = 0;
        
        // TODO: Implement actual schema reset logic
        // This would reset the Arrow schema for better compression
        
        Ok(())
    }
}

#[async_trait]
impl ProtocolHandler for OtapProtocol {
    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing OTAP protocol handler");

        // Validate configuration
        self.config.validate().await?;

        // Initialize gRPC server
        self.init_server().await?;

        info!("OTAP protocol handler initialized");
        Ok(())
    }

    async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting OTAP protocol handler");

        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = true;
        }

        // Start gRPC server
        self.start_server().await?;

        info!("OTAP protocol handler started");
        Ok(())
    }

    async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping OTAP protocol handler");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = false;
        }

        info!("OTAP protocol handler stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        // This is a simplified check - in practice we'd need to handle the async nature
        false
    }

    async fn get_stats(&self) -> BridgeResult<ProtocolStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn receive_data(&self) -> BridgeResult<Option<TelemetryBatch>> {
        // Get mutable access to the data receiver
        let mut data_rx_guard = self.data_rx.lock().await;
        
        // Check if we have a data receiver
        if let Some(data_rx) = data_rx_guard.as_mut() {
            // Try to receive data from the channel (non-blocking)
            match data_rx.try_recv() {
                Ok(batch) => {
                    // Update statistics
                    {
                        let mut stats = self.stats.write().await;
                        stats.total_messages += 1;
                        stats.total_bytes += batch.records.iter().map(|r| r.attributes.len()).sum::<usize>() as u64;
                        stats.last_message_time = Some(Utc::now());
                    }
                    Ok(Some(batch))
                }
                Err(TryRecvError::Empty) => {
                    // No data available
                    Ok(None)
                }
                Err(TryRecvError::Disconnected) => {
                    // Channel is disconnected
                    Ok(None)
                }
            }
        } else {
            // No data receiver available
            Ok(None)
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// OTAP message handler
pub struct OtapMessageHandler;

impl OtapMessageHandler {
    pub fn new() -> Self {
        Self
    }

    /// Decode OTAP Arrow message
    async fn decode_otap_message(&self, payload: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        // TODO: Implement actual OTAP message decoding using otel-arrow-rust
        // This would decode the Arrow IPC format and convert to TelemetryRecord
        
        info!("Decoding OTAP Arrow message of {} bytes", payload.len());

        // For now, create a placeholder record
        let records = vec![
            TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name: "decoded_otap_metric".to_string(),
                    description: Some("Decoded from OTAP Arrow format".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: bridge_core::types::MetricType::Gauge,
                    value: MetricValue::Gauge(123.45),
                    labels: HashMap::from([
                        ("source".to_string(), "otap_decoder".to_string()),
                        ("payload_size".to_string(), payload.len().to_string()),
                    ]),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::from([
                    ("protocol".to_string(), "otap".to_string()),
                    ("decoder".to_string(), "arrow".to_string()),
                ]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            },
        ];

        Ok(records)
    }
}

#[async_trait]
impl MessageHandler for OtapMessageHandler {
    async fn handle_message(&self, message: ProtocolMessage) -> BridgeResult<TelemetryBatch> {
        let records = match message.payload {
            super::MessagePayload::Arrow(data) => {
                self.decode_otap_message(&data).await?
            }
            super::MessagePayload::Json(data) => {
                // Fallback to JSON processing if needed
                warn!("Received JSON message in OTAP handler, falling back to JSON processing");
                self.decode_otap_message(&data).await?
            }
            super::MessagePayload::Protobuf(data) => {
                // Fallback to protobuf processing if needed
                warn!("Received protobuf message in OTAP handler, falling back to protobuf processing");
                self.decode_otap_message(&data).await?
            }
        };

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "otap-message-handler".to_string(),
            size: records.len(),
            records,
            metadata: message.metadata,
        })
    }

    async fn handle_batch(
        &self,
        messages: Vec<ProtocolMessage>,
    ) -> BridgeResult<Vec<TelemetryBatch>> {
        let mut batches = Vec::new();
        
        for message in messages {
            let batch = self.handle_message(message).await?;
            batches.push(batch);
        }
        
        Ok(batches)
    }
}
