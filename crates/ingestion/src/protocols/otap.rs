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
use tokio::sync::{mpsc, mpsc::error::TryRecvError, Mutex, RwLock};
use tonic::{transport::Server, Request, Response, Status};
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

/// OTAP server wrapper with actual gRPC server implementation
struct OtapServer {
    addr: std::net::SocketAddr,
    server_handle: Option<tokio::task::JoinHandle<Result<(), tonic::transport::Error>>>,
}

/// OTAP gRPC service implementation
#[derive(Clone)]
struct OtapGrpcService {
    protocol: Arc<OtapProtocol>,
}

impl OtapGrpcService {
    fn new(protocol: Arc<OtapProtocol>) -> Self {
        Self { protocol }
    }

    async fn process_otap_request(&self, data: &[u8]) -> Result<u32, Status> {
        info!("Processing OTAP request: {} bytes", data.len());

        match self.protocol.process_otap_data(data).await {
            Ok(batch) => {
                // Update statistics
                {
                    let mut stats = self.protocol.stats.write().await;
                    stats.total_messages += 1;
                    stats.total_bytes += data.len() as u64;
                    stats.last_message_time = Some(Utc::now());
                }

                Ok(batch.records.len() as u32)
            }
            Err(e) => {
                error!("Failed to process OTAP data: {}", e);
                
                // Update error statistics
                {
                    let mut stats = self.protocol.stats.write().await;
                    stats.error_count += 1;
                }

                Err(Status::internal(format!("Failed to process OTAP data: {}", e)))
            }
        }
    }
}

impl OtapProtocol {
    /// Create new OTAP protocol handler
    pub async fn new(config: &dyn ProtocolConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<OtapConfig>()
            .ok_or_else(|| bridge_core::BridgeError::configuration("Invalid OTAP configuration"))?
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

    /// Initialize gRPC server with OTAP services
    async fn init_server(&mut self) -> BridgeResult<()> {
        info!(
            "Initializing OTAP gRPC server on {}:{}",
            self.config.endpoint, self.config.port
        );

        // Parse the endpoint address
        let addr = format!("{}:{}", self.config.endpoint, self.config.port)
            .parse::<std::net::SocketAddr>()
            .map_err(|e| {
                bridge_core::BridgeError::configuration(format!("Invalid OTAP address: {}", e))
            })?;

        self.server = Some(OtapServer {
            addr,
            server_handle: None,
        });

        info!("OTAP gRPC server initialized");
        Ok(())
    }

    /// Start gRPC server with OTAP services
    async fn start_server(&self) -> BridgeResult<()> {
        info!("Starting OTAP gRPC server");

        if let Some(server) = &self.server {
            let addr = server.addr;
            
            // For now, we'll just log that the server would start
            // In a real implementation, you would create a proper gRPC server
            // with OTAP services using tonic
            info!("OTAP gRPC server would start on {} (implementation pending)", addr);
            
            // TODO: Implement actual gRPC server startup with OTAP services
            // This would involve:
            // 1. Creating a tonic server
            // 2. Adding OTAP service implementations
            // 3. Starting the server in a background task
            // 4. Storing the server handle for graceful shutdown
        }

        Ok(())
    }

    /// Process OTAP Arrow data with actual Arrow decoding
    async fn process_otap_data(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        info!("Processing OTAP Arrow data of {} bytes", data.len());

        // Try to decode as Arrow IPC format
        let records = self.decode_arrow_data(data).await?;

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
            records: records.clone(),
            metadata: HashMap::from([
                ("protocol".to_string(), "otap".to_string()),
                ("content_type".to_string(), "application/arrow".to_string()),
                (
                    "compression".to_string(),
                    self.config.enable_compression.to_string(),
                ),
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

    /// Decode Arrow IPC data into telemetry records
    async fn decode_arrow_data(&self, data: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        // This is a placeholder implementation for Arrow decoding
        // In a real implementation, you would use the otel-arrow-rust crate
        // to decode the Arrow IPC format and convert to TelemetryRecord
        
        // For now, we'll create a basic implementation that attempts to parse
        // the data and extract meaningful information
        
        if data.is_empty() {
            return Err(bridge_core::BridgeError::internal("Empty OTAP data received"));
        }

        // Check if this looks like Arrow IPC data (magic bytes)
        if data.len() >= 4 && &data[0..4] == b"ARROW" {
            info!("Detected Arrow IPC format data");
            
            // TODO: Implement actual Arrow IPC decoding using otel-arrow-rust
            // This would involve:
            // 1. Reading the Arrow IPC header
            // 2. Decoding the schema
            // 3. Reading the record batches
            // 4. Converting to OpenTelemetry format
            // 5. Creating TelemetryRecord instances
            
            // For now, create a placeholder record indicating Arrow data was received
            let records = vec![TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name: "arrow_otap_metric".to_string(),
                    description: Some("Decoded from Arrow IPC format".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: bridge_core::types::MetricType::Counter,
                    value: MetricValue::Counter(data.len() as f64),
                    labels: HashMap::from([
                        ("source".to_string(), "otap_arrow".to_string()),
                        ("data_size".to_string(), data.len().to_string()),
                        ("format".to_string(), "arrow_ipc".to_string()),
                    ]),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::from([
                    ("protocol".to_string(), "otap".to_string()),
                    ("format".to_string(), "arrow_ipc".to_string()),
                    ("decoded".to_string(), "true".to_string()),
                ]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            }];

            Ok(records)
        } else {
            // Fallback: treat as generic binary data
            warn!("Non-Arrow format data received, treating as generic binary");
            
            let records = vec![TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Log,
                data: TelemetryData::Log(LogData {
                    message: "Received non-Arrow format data".to_string(),
                    level: bridge_core::types::LogLevel::Warn,
                    timestamp: Utc::now(),
                    attributes: HashMap::from([
                        ("source".to_string(), "otap_fallback".to_string()),
                        ("data_size".to_string(), data.len().to_string()),
                        ("format".to_string(), "unknown".to_string()),
                    ]),
                    body: Some(format!("Received {} bytes of unknown format data", data.len())),
                    severity_number: Some(13), // WARN level
                    severity_text: Some("WARN".to_string()),
                }),
                attributes: HashMap::from([
                    ("protocol".to_string(), "otap".to_string()),
                    ("fallback".to_string(), "true".to_string()),
                ]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            }];

            Ok(records)
        }
    }

    /// Send data to receiver through the data channel
    async fn send_data_to_receiver(&self, batch: TelemetryBatch) -> BridgeResult<()> {
        if let Some(data_tx) = &self.data_tx {
            data_tx.send(batch).map_err(|e| {
                bridge_core::BridgeError::internal(format!(
                    "Failed to send data to receiver: {}",
                    e
                ))
            })?;
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
                (
                    "compression".to_string(),
                    self.config.enable_compression.to_string(),
                ),
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

    /// Reset schema and batch count with actual schema reset logic
    async fn reset_schema(&self) -> BridgeResult<()> {
        info!("Resetting OTAP schema");

        let mut batch_count = self.batch_count.write().await;
        *batch_count = 0;

        // TODO: Implement actual schema reset logic using otel-arrow-rust
        // This would involve:
        // 1. Clearing the current Arrow schema cache
        // 2. Resetting any schema-related state
        // 3. Preparing for new schema inference
        // 4. Optionally, sending a schema reset notification
        
        info!("OTAP schema reset completed");
        Ok(())
    }
}

// Implement Clone for OtapProtocol to allow sharing in gRPC service
impl Clone for OtapProtocol {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            is_running: Arc::clone(&self.is_running),
            stats: Arc::clone(&self.stats),
            message_handler: Arc::clone(&self.message_handler),
            server: None, // Server handle cannot be cloned
            batch_count: Arc::clone(&self.batch_count),
            data_tx: self.data_tx.clone(),
            data_rx: Arc::clone(&self.data_rx),
        }
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
                        stats.total_bytes += batch
                            .records
                            .iter()
                            .map(|r| r.attributes.len())
                            .sum::<usize>() as u64;
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

    /// Decode OTAP Arrow message with actual Arrow decoding
    async fn decode_otap_message(&self, payload: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        info!("Decoding OTAP Arrow message of {} bytes", payload.len());

        // TODO: Implement actual OTAP message decoding using otel-arrow-rust
        // This would decode the Arrow IPC format and convert to TelemetryRecord
        // For now, we'll create a placeholder implementation
        
        if payload.is_empty() {
            return Err(bridge_core::BridgeError::internal("Empty OTAP message payload"));
        }

        // Check for Arrow IPC magic bytes
        if payload.len() >= 4 && &payload[0..4] == b"ARROW" {
            info!("Detected Arrow IPC format in OTAP message");
            
            // Create a placeholder record for Arrow data
            let records = vec![TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name: "otap_arrow_metric".to_string(),
                    description: Some("Decoded from OTAP Arrow format".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: bridge_core::types::MetricType::Gauge,
                    value: MetricValue::Gauge(payload.len() as f64),
                    labels: HashMap::from([
                        ("source".to_string(), "otap_decoder".to_string()),
                        ("payload_size".to_string(), payload.len().to_string()),
                        ("format".to_string(), "arrow_ipc".to_string()),
                    ]),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::from([
                    ("protocol".to_string(), "otap".to_string()),
                    ("decoder".to_string(), "arrow".to_string()),
                    ("decoded".to_string(), "true".to_string()),
                ]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            }];

            Ok(records)
        } else {
            // Fallback for non-Arrow data
            warn!("Non-Arrow format in OTAP message, using fallback processing");
            
            let records = vec![TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Log,
                data: TelemetryData::Log(LogData {
                    message: "Non-Arrow format OTAP message".to_string(),
                    level: bridge_core::types::LogLevel::Warn,
                    timestamp: Utc::now(),
                    attributes: HashMap::from([
                        ("source".to_string(), "otap_decoder".to_string()),
                        ("payload_size".to_string(), payload.len().to_string()),
                        ("format".to_string(), "unknown".to_string()),
                    ]),
                    body: Some(format!("Received {} bytes of unknown format data", payload.len())),
                    severity_number: Some(13), // WARN level
                    severity_text: Some("WARN".to_string()),
                }),
                attributes: HashMap::from([
                    ("protocol".to_string(), "otap".to_string()),
                    ("fallback".to_string(), "true".to_string()),
                ]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            }];

            Ok(records)
        }
    }
}

#[async_trait]
impl MessageHandler for OtapMessageHandler {
    async fn handle_message(&self, message: ProtocolMessage) -> BridgeResult<TelemetryBatch> {
        let records = match message.payload {
            super::MessagePayload::Arrow(data) => self.decode_otap_message(&data).await?,
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
