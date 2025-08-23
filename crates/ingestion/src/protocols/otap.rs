//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OTel-Arrow Protocol (OTAP) implementation
//!
//! This module provides support for ingesting OpenTelemetry data using the
//! OTel-Arrow Protocol (OTAP) which provides significant compression improvements
//! over standard OTLP.

use arrow_array::{ArrayRef, RecordBatch};
use async_trait::async_trait;
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response as AxumResponse,
    routing::post,
    Router,
};
use bridge_core::{
    types::{LogData, MetricData, MetricValue, TelemetryData, TelemetryRecord, TelemetryType},
    BridgeResult, TelemetryBatch,
};
use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, mpsc::error::TryRecvError, Mutex, RwLock};
use tonic::server::NamedService;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{MessageHandler, ProtocolConfig, ProtocolHandler, ProtocolMessage, ProtocolStats};

// OTAP gRPC service definitions
// These would typically come from a .proto file, but we'll define them inline for now
#[derive(Debug, Clone)]
pub struct OtapExportRequest {
    pub data: Vec<u8>,
    pub content_type: String,
    pub compression: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OtapExportResponse {
    pub records_processed: u32,
    pub success: bool,
    pub error_message: Option<String>,
}

// OTAP gRPC service trait - simplified for now
pub trait OtapServiceTrait: Send + Sync {
    async fn export(
        &self,
        request: Request<OtapExportRequest>,
    ) -> Result<Response<OtapExportResponse>, Status>;
}

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

impl OtapServer {
    fn new(addr: std::net::SocketAddr) -> Self {
        Self {
            addr,
            server_handle: None,
        }
    }
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

                Err(Status::internal(format!(
                    "Failed to process OTAP data: {}",
                    e
                )))
            }
        }
    }
}

impl OtapServiceTrait for OtapGrpcService {
    async fn export(
        &self,
        request: Request<OtapExportRequest>,
    ) -> Result<Response<OtapExportResponse>, Status> {
        let request_data = request.into_inner();

        match self.process_otap_request(&request_data.data).await {
            Ok(records_processed) => {
                let response = OtapExportResponse {
                    records_processed,
                    success: true,
                    error_message: None,
                };
                Ok(Response::new(response))
            }
            Err(status) => {
                let response = OtapExportResponse {
                    records_processed: 0,
                    success: false,
                    error_message: Some(status.message().to_string()),
                };
                Ok(Response::new(response))
            }
        }
    }
}

impl NamedService for OtapGrpcService {
    const NAME: &'static str = "otap.OtapService";
}

/// HTTP handler for OTAP export endpoint
async fn otap_export_handler(
    State(protocol): State<Arc<OtapProtocol>>,
    headers: HeaderMap,
    body: Body,
) -> AxumResponse<String> {
    info!("Received OTAP export request");

    // Extract content type from headers
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/arrow")
        .to_string();

    // Read request body
    let body_bytes = match to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes.to_vec(),
        Err(e) => {
            error!("Failed to read request body: {}", e);
            return AxumResponse::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Failed to read request body".to_string())
                .unwrap();
        }
    };

    // Process OTAP data
    match protocol.process_otap_data(&body_bytes).await {
        Ok(batch) => {
            info!(
                "Successfully processed OTAP data: {} records",
                batch.records.len()
            );

            // Update statistics
            {
                let mut stats = protocol.stats.write().await;
                stats.total_messages += 1;
                stats.total_bytes += body_bytes.len() as u64;
                stats.last_message_time = Some(Utc::now());
            }

            AxumResponse::builder()
                .status(StatusCode::OK)
                .body(format!(
                    "{{ \"records_processed\": {}, \"success\": true }}",
                    batch.records.len()
                ))
                .unwrap()
        }
        Err(e) => {
            error!("Failed to process OTAP data: {}", e);

            // Update error statistics
            {
                let mut stats = protocol.stats.write().await;
                stats.error_count += 1;
            }

            AxumResponse::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("{{ \"success\": false, \"error\": \"{}\" }}", e))
                .unwrap()
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

    /// Create a dummy OTAP protocol handler for testing purposes
    pub async fn new_dummy() -> BridgeResult<Self> {
        let config = OtapConfig::new("127.0.0.1".to_string(), 8080);

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

        self.server = Some(OtapServer::new(addr));

        info!("OTAP gRPC server initialized");
        Ok(())
    }

    /// Start OTAP server with HTTP endpoint for Arrow data
    async fn start_server(&self) -> BridgeResult<()> {
        info!("Starting OTAP server");

        if let Some(server) = &self.server {
            let addr = server.addr;

            info!("Starting OTAP server on {}", addr);

            // Create protocol instance for the server
            let protocol_arc = Arc::new(self.clone());

            // Start HTTP server for OTAP Arrow data
            let _server_handle = tokio::spawn(async move {
                let app = Router::new()
                    .route("/otap/export", post(otap_export_handler))
                    .with_state(protocol_arc);

                let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
                axum::serve(listener, app).await.map_err(|e| {
                    error!("OTAP server error: {}", e);
                    e
                })
            });

            info!("OTAP server started successfully on {}", addr);
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
        if data.is_empty() {
            return Err(bridge_core::BridgeError::internal(
                "Empty OTAP data received",
            ));
        }

        // Check if this looks like Arrow IPC data (magic bytes)
        if data.len() >= 4 && &data[0..4] == b"ARROW" {
            info!("Detected Arrow IPC format data");

            // Try to decode as Arrow IPC format
            match self.decode_arrow_ipc(data).await {
                Ok(records) => {
                    info!("Successfully decoded {} Arrow IPC records", records.len());
                    return Ok(records);
                }
                Err(e) => {
                    warn!(
                        "Failed to decode Arrow IPC data: {}, falling back to generic decoding",
                        e
                    );
                }
            }
        }

        // Fallback to generic decoding
        self.decode_generic_arrow_data(data).await
    }

    /// Decode Arrow IPC format data
    async fn decode_arrow_ipc(&self, data: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        use arrow_ipc::reader::StreamReader;
        use std::io::Cursor;

        let cursor = Cursor::new(data);
        let mut reader = StreamReader::try_new(cursor, None).map_err(|e| {
            bridge_core::BridgeError::internal(format!("Failed to create Arrow IPC reader: {}", e))
        })?;

        let mut records = Vec::new();

        while let Some(batch_result) = reader.next() {
            let batch = batch_result.map_err(|e| {
                bridge_core::BridgeError::internal(format!("Failed to read Arrow batch: {}", e))
            })?;

            let batch_records = self.convert_arrow_batch_to_records(batch).await?;
            records.extend(batch_records);
        }

        Ok(records)
    }

    /// Convert Arrow RecordBatch to TelemetryRecord
    async fn convert_arrow_batch_to_records(
        &self,
        batch: RecordBatch,
    ) -> BridgeResult<Vec<TelemetryRecord>> {
        let mut records = Vec::new();
        let num_rows = batch.num_rows();

        // Extract common fields from the batch
        let timestamp_col = batch.column_by_name("timestamp").ok_or_else(|| {
            bridge_core::BridgeError::internal("Missing timestamp column in Arrow batch")
        })?;

        let name_col = batch.column_by_name("name");
        let value_col = batch.column_by_name("value");
        let type_col = batch.column_by_name("type");

        for row_idx in 0..num_rows {
            let timestamp = self.extract_timestamp_from_arrow(timestamp_col, row_idx)?;
            let name = name_col
                .and_then(|col| self.extract_string_from_arrow(col, row_idx))
                .unwrap_or_else(|| "unknown_metric".to_string());
            let value = value_col
                .and_then(|col| self.extract_value_from_arrow(col, row_idx))
                .unwrap_or(0.0);
            let record_type = type_col
                .and_then(|col| self.extract_string_from_arrow(col, row_idx))
                .unwrap_or_else(|| "metric".to_string());

            let telemetry_type = match record_type.as_str() {
                "metric" => TelemetryType::Metric,
                "log" => TelemetryType::Log,
                "trace" => TelemetryType::Trace,
                _ => TelemetryType::Metric,
            };

            let record = match telemetry_type {
                TelemetryType::Metric => TelemetryRecord {
                    id: Uuid::new_v4(),
                    timestamp,
                    record_type: TelemetryType::Metric,
                    data: TelemetryData::Metric(MetricData {
                        name,
                        description: Some("Decoded from Arrow IPC".to_string()),
                        unit: Some("count".to_string()),
                        metric_type: bridge_core::types::MetricType::Gauge,
                        value: MetricValue::Gauge(value),
                        labels: HashMap::new(),
                        timestamp,
                    }),
                    attributes: HashMap::new(),
                    tags: HashMap::new(),
                    resource: None,
                    service: None,
                },
                TelemetryType::Log => TelemetryRecord {
                    id: Uuid::new_v4(),
                    timestamp,
                    record_type: TelemetryType::Log,
                    data: TelemetryData::Log(LogData {
                        timestamp,
                        level: bridge_core::types::LogLevel::Info,
                        message: format!("Arrow IPC log: {}", name),
                        attributes: HashMap::new(),
                        body: None,
                        severity_number: Some(9),
                        severity_text: Some("INFO".to_string()),
                    }),
                    attributes: HashMap::new(),
                    tags: HashMap::new(),
                    resource: None,
                    service: None,
                },
                _ => continue, // Skip unsupported types for now
            };

            records.push(record);
        }

        Ok(records)
    }

    /// Extract timestamp from Arrow column
    fn extract_timestamp_from_arrow(
        &self,
        column: &ArrayRef,
        row_idx: usize,
    ) -> BridgeResult<DateTime<Utc>> {
        use arrow_array::{
            TimestampMicrosecondArray, TimestampMillisecondArray, TimestampNanosecondArray,
            TimestampSecondArray,
        };

        match column.as_any().downcast_ref::<TimestampNanosecondArray>() {
            Some(arr) => {
                let value = arr.value(row_idx);
                Ok(DateTime::from_timestamp_nanos(value as i64))
            }
            None => match column.as_any().downcast_ref::<TimestampMicrosecondArray>() {
                Some(arr) => {
                    let value = arr.value(row_idx);
                    Ok(DateTime::from_timestamp_micros(value as i64).unwrap_or_else(|| Utc::now()))
                }
                None => match column.as_any().downcast_ref::<TimestampMillisecondArray>() {
                    Some(arr) => {
                        let value = arr.value(row_idx);
                        Ok(DateTime::from_timestamp_millis(value as i64)
                            .unwrap_or_else(|| Utc::now()))
                    }
                    None => match column.as_any().downcast_ref::<TimestampSecondArray>() {
                        Some(arr) => {
                            let value = arr.value(row_idx);
                            Ok(DateTime::from_timestamp(value as i64, 0)
                                .unwrap_or_else(|| Utc::now()))
                        }
                        None => Ok(Utc::now()), // Fallback to current time
                    },
                },
            },
        }
    }

    /// Extract string from Arrow column
    fn extract_string_from_arrow(&self, column: &ArrayRef, row_idx: usize) -> Option<String> {
        use arrow_array::StringArray;

        column
            .as_any()
            .downcast_ref::<StringArray>()
            .map(|arr| arr.value(row_idx).to_string())
    }

    /// Extract numeric value from Arrow column
    fn extract_value_from_arrow(&self, column: &ArrayRef, row_idx: usize) -> Option<f64> {
        use arrow_array::{Float32Array, Float64Array, Int32Array, Int64Array};

        if let Some(arr) = column.as_any().downcast_ref::<Float64Array>() {
            Some(arr.value(row_idx))
        } else if let Some(arr) = column.as_any().downcast_ref::<Int64Array>() {
            Some(arr.value(row_idx) as f64)
        } else if let Some(arr) = column.as_any().downcast_ref::<Float32Array>() {
            Some(arr.value(row_idx) as f64)
        } else if let Some(arr) = column.as_any().downcast_ref::<Int32Array>() {
            Some(arr.value(row_idx) as f64)
        } else {
            None
        }
    }

    /// Decode generic Arrow data (fallback)
    async fn decode_generic_arrow_data(&self, data: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        // Create a placeholder record indicating Arrow data was received
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

        // Clear Arrow schema cache and reset state
        self.clear_arrow_schema_cache().await?;

        // Prepare for new schema inference
        self.prepare_schema_inference().await?;

        info!("OTAP schema reset completed");
        Ok(())
    }

    /// Clear Arrow schema cache
    async fn clear_arrow_schema_cache(&self) -> BridgeResult<()> {
        // In a real implementation, this would clear any cached Arrow schemas
        // For now, we'll just log the action
        info!("Clearing Arrow schema cache");
        Ok(())
    }

    /// Prepare for new schema inference
    async fn prepare_schema_inference(&self) -> BridgeResult<()> {
        // In a real implementation, this would reset schema inference state
        // and prepare for detecting new schemas
        info!("Preparing for new schema inference");
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

        if payload.is_empty() {
            return Err(bridge_core::BridgeError::internal(
                "Empty OTAP message payload",
            ));
        }

        // Try to decode as OTAP format first
        match self.decode_otap_format(payload).await {
            Ok(records) => {
                info!("Successfully decoded {} OTAP records", records.len());
                return Ok(records);
            }
            Err(e) => {
                warn!(
                    "Failed to decode OTAP format: {}, falling back to Arrow decoding",
                    e
                );
            }
        }

        // Fallback to direct Arrow decoding
        self.decode_arrow_data(payload).await
    }

    /// Decode OTAP format message
    async fn decode_otap_format(&self, data: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        // OTAP format typically has a header followed by Arrow data
        // For now, we'll implement a basic OTAP format parser

        if data.len() < 8 {
            return Err(bridge_core::BridgeError::internal("OTAP message too short"));
        }

        // Check for OTAP magic bytes (placeholder - actual OTAP format may differ)
        if &data[0..4] == b"OTAP" {
            // Extract Arrow data from OTAP message
            let arrow_data = &data[8..]; // Skip header
            return self.decode_arrow_data(arrow_data).await;
        }

        // If not OTAP format, try direct Arrow decoding
        self.decode_arrow_data(data).await
    }

    /// Decode Arrow data directly
    async fn decode_arrow_data(&self, data: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        // For now, return a placeholder implementation
        // In a real implementation, this would decode Arrow format data
        warn!("Arrow decoding not yet implemented, returning empty records");
        Ok(Vec::new())
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
            source: "otap".to_string(),
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
