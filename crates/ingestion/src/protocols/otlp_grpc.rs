//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OTLP gRPC protocol implementation
//!
//! This module provides support for ingesting OpenTelemetry data over gRPC
//! using the standard OTLP protocol.

use ::prost::Message;
use async_trait::async_trait;
use bridge_core::types::{
    LogData, MetricData, MetricValue, TelemetryData, TelemetryRecord, TelemetryType,
};
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use opentelemetry_proto::tonic::collector::{
    logs::v1::logs_service_server::LogsService,
    logs::v1::{ExportLogsServiceRequest, ExportLogsServiceResponse},
    metrics::v1::metrics_service_server::MetricsService,
    metrics::v1::{ExportMetricsServiceRequest, ExportMetricsServiceResponse},
};
use opentelemetry_proto::tonic::common::v1::any_value::Value as AnyValueValue;
use opentelemetry_proto::tonic::metrics::v1::metric::Data as OtlpMetricData;
use opentelemetry_proto::tonic::metrics::v1::number_data_point::Value as NumberDataPointValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{transport::Server, Response, Status};
use tracing::{error, info};
use uuid::Uuid;

use super::{MessageHandler, ProtocolConfig, ProtocolHandler, ProtocolMessage, ProtocolStats};

/// OTLP gRPC protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpGrpcConfig {
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
}

impl OtlpGrpcConfig {
    /// Create new OTLP gRPC configuration
    pub fn new(endpoint: String, port: u16) -> Self {
        Self {
            name: "otlp-grpc".to_string(),
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
        }
    }
}

#[async_trait]
impl ProtocolConfig for OtlpGrpcConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.endpoint.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "OTLP gRPC endpoint cannot be empty",
            ));
        }

        if self.port == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "OTLP gRPC port cannot be 0",
            ));
        }

        if self.batch_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "OTLP gRPC batch size cannot be 0",
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

/// OTLP gRPC protocol handler
#[derive(Clone)]
pub struct OtlpGrpcProtocol {
    config: OtlpGrpcConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ProtocolStats>>,
    message_handler: Arc<dyn MessageHandler>,
    server: Option<OtlpGrpcServer>,
}

/// OTLP gRPC server wrapper
#[derive(Clone)]
struct OtlpGrpcServer {
    addr: std::net::SocketAddr,
    metrics_service: OtlpMetricsService,
    logs_service: OtlpLogsService,
}

impl OtlpGrpcProtocol {
    /// Create new OTLP gRPC protocol handler
    pub async fn new(config: &dyn ProtocolConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<OtlpGrpcConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration("Invalid OTLP gRPC configuration")
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

        Ok(Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
            message_handler: Arc::new(OtlpGrpcMessageHandler::new()),
            server: None,
        })
    }

    /// Initialize gRPC server
    async fn init_server(&mut self) -> BridgeResult<()> {
        info!(
            "Initializing OTLP gRPC server on {}:{}",
            self.config.endpoint, self.config.port
        );

        // Parse the endpoint address
        let addr = format!("{}:{}", self.config.endpoint, self.config.port)
            .parse::<std::net::SocketAddr>()
            .map_err(|e| {
                bridge_core::BridgeError::configuration(format!("Invalid OTLP gRPC address: {}", e))
            })?;

        // Create service instances
        let protocol_arc = Arc::new(self.clone());
        let metrics_service = OtlpMetricsService {
            protocol: Arc::clone(&protocol_arc),
        };
        let logs_service = OtlpLogsService {
            protocol: Arc::clone(&protocol_arc),
        };

        self.server = Some(OtlpGrpcServer {
            addr,
            metrics_service,
            logs_service,
        });

        info!("OTLP gRPC server initialized");
        Ok(())
    }

    /// Start gRPC server
    async fn start_server(&self) -> BridgeResult<()> {
        info!("Starting OTLP gRPC server");

        if let Some(server_wrapper) = &self.server {
            // Create the tonic server with OTLP services (metrics and logs)
            let server = Server::builder()
                .add_service(
                    opentelemetry_proto::tonic::collector::metrics::v1::metrics_service_server::MetricsServiceServer::new(
                        server_wrapper.metrics_service.clone()
                    )
                )
                .add_service(
                    opentelemetry_proto::tonic::collector::logs::v1::logs_service_server::LogsServiceServer::new(
                        server_wrapper.logs_service.clone()
                    )
                );

            // Start the server in a separate task
            let addr = server_wrapper.addr;
            tokio::spawn(async move {
                match server.serve(addr).await {
                    Ok(_) => info!("OTLP gRPC server stopped"),
                    Err(e) => error!("OTLP gRPC server error: {}", e),
                }
            });

            info!("OTLP gRPC server started on {}", addr);
        }

        Ok(())
    }

    /// Process OTLP metrics request
    async fn process_metrics_request(
        &self,
        request: ExportMetricsServiceRequest,
    ) -> BridgeResult<TelemetryBatch> {
        // Implement metrics processing
        // Convert OTLP metrics to TelemetryBatch

        let mut records = Vec::new();

        for resource_metrics in request.resource_metrics {
            let resource_attrs: HashMap<String, String> = resource_metrics
                .resource
                .map(|r| {
                    r.attributes
                        .into_iter()
                        .map(|attr| {
                            (
                                attr.key,
                                attr.value
                                    .map(|v| match v.value {
                                        Some(AnyValueValue::StringValue(s)) => s,
                                        Some(AnyValueValue::IntValue(i)) => i.to_string(),
                                        Some(AnyValueValue::DoubleValue(d)) => d.to_string(),
                                        Some(AnyValueValue::BoolValue(b)) => b.to_string(),
                                        _ => "unknown".to_string(),
                                    })
                                    .unwrap_or_default(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();

            for scope_metrics in resource_metrics.scope_metrics {
                for metric in scope_metrics.metrics {
                    let metric_name = metric.name;
                    let metric_description = metric.description;
                    let metric_unit = metric.unit;

                    // Convert OTLP metric data points to our format
                    for data_point in metric.data {
                        match data_point {
                            OtlpMetricData::Gauge(gauge) => {
                                for point in gauge.data_points {
                                    let value = match point.value {
                                        Some(NumberDataPointValue::AsDouble(v)) => {
                                            MetricValue::Gauge(v)
                                        }
                                        Some(NumberDataPointValue::AsInt(v)) => {
                                            MetricValue::Gauge(v as f64)
                                        }
                                        None => MetricValue::Gauge(0.0),
                                    };

                                    let labels: HashMap<String, String> = point
                                        .attributes
                                        .into_iter()
                                        .map(|attr| {
                                            (
                                                attr.key,
                                                attr.value
                                                    .map(|v| match v.value {
                                                        Some(AnyValueValue::StringValue(s)) => s,
                                                        Some(AnyValueValue::IntValue(i)) => {
                                                            i.to_string()
                                                        }
                                                        Some(AnyValueValue::DoubleValue(d)) => {
                                                            d.to_string()
                                                        }
                                                        Some(AnyValueValue::BoolValue(b)) => {
                                                            b.to_string()
                                                        }
                                                        _ => "unknown".to_string(),
                                                    })
                                                    .unwrap_or_default(),
                                            )
                                        })
                                        .collect();

                                    let timestamp = if point.time_unix_nano > 0 {
                                        DateTime::from_timestamp_nanos(point.time_unix_nano as i64)
                                    } else {
                                        Utc::now()
                                    };

                                    records.push(TelemetryRecord {
                                        id: Uuid::new_v4(),
                                        timestamp,
                                        record_type: TelemetryType::Metric,
                                        data: TelemetryData::Metric(MetricData {
                                            name: metric_name.clone(),
                                            description: Some(metric_description.clone()),
                                            unit: Some(metric_unit.clone()),
                                            metric_type: bridge_core::types::MetricType::Gauge,
                                            value,
                                            labels,
                                            timestamp,
                                        }),
                                        attributes: resource_attrs.clone(),
                                        tags: HashMap::new(),
                                        resource: None,
                                        service: None,
                                    });
                                }
                            }
                            OtlpMetricData::Sum(sum) => {
                                for point in sum.data_points {
                                    let value = match point.value {
                                        Some(NumberDataPointValue::AsDouble(v)) => {
                                            MetricValue::Counter(v)
                                        }
                                        Some(NumberDataPointValue::AsInt(v)) => {
                                            MetricValue::Counter(v as f64)
                                        }
                                        None => MetricValue::Counter(0.0),
                                    };

                                    let labels: HashMap<String, String> = point
                                        .attributes
                                        .into_iter()
                                        .map(|attr| {
                                            (
                                                attr.key,
                                                attr.value
                                                    .map(|v| match v.value {
                                                        Some(AnyValueValue::StringValue(s)) => s,
                                                        Some(AnyValueValue::IntValue(i)) => {
                                                            i.to_string()
                                                        }
                                                        Some(AnyValueValue::DoubleValue(d)) => {
                                                            d.to_string()
                                                        }
                                                        Some(AnyValueValue::BoolValue(b)) => {
                                                            b.to_string()
                                                        }
                                                        _ => "unknown".to_string(),
                                                    })
                                                    .unwrap_or_default(),
                                            )
                                        })
                                        .collect();

                                    let timestamp = if point.time_unix_nano > 0 {
                                        DateTime::from_timestamp_nanos(point.time_unix_nano as i64)
                                    } else {
                                        Utc::now()
                                    };

                                    records.push(TelemetryRecord {
                                        id: Uuid::new_v4(),
                                        timestamp,
                                        record_type: TelemetryType::Metric,
                                        data: TelemetryData::Metric(MetricData {
                                            name: metric_name.clone(),
                                            description: Some(metric_description.clone()),
                                            unit: Some(metric_unit.clone()),
                                            metric_type: bridge_core::types::MetricType::Counter,
                                            value,
                                            labels,
                                            timestamp,
                                        }),
                                        attributes: resource_attrs.clone(),
                                        tags: HashMap::new(),
                                        resource: None,
                                        service: None,
                                    });
                                }
                            }
                            _ => {
                                // Handle other metric types (Histogram, Summary, etc.) as needed
                                info!("Unsupported metric type for metric: {}", metric_name);
                            }
                        }
                    }
                }
            }
        }

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "otlp-grpc".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::from([
                ("protocol".to_string(), "otlp-grpc".to_string()),
                ("content_type".to_string(), "application/grpc".to_string()),
            ]),
        })
    }

    /// Process OTLP logs request
    async fn process_logs_request(
        &self,
        request: ExportLogsServiceRequest,
    ) -> BridgeResult<TelemetryBatch> {
        // Implement logs processing
        // Convert OTLP logs to TelemetryBatch

        let mut records = Vec::new();

        for resource_logs in request.resource_logs {
            let resource_attrs: HashMap<String, String> = resource_logs
                .resource
                .map(|r| {
                    r.attributes
                        .into_iter()
                        .map(|attr| {
                            (
                                attr.key,
                                attr.value
                                    .map(|v| match v.value {
                                        Some(AnyValueValue::StringValue(s)) => s,
                                        Some(AnyValueValue::IntValue(i)) => i.to_string(),
                                        Some(AnyValueValue::DoubleValue(d)) => d.to_string(),
                                        Some(AnyValueValue::BoolValue(b)) => b.to_string(),
                                        _ => "unknown".to_string(),
                                    })
                                    .unwrap_or_default(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();

            for scope_logs in resource_logs.scope_logs {
                for log_record in scope_logs.log_records {
                    let timestamp = if log_record.time_unix_nano > 0 {
                        DateTime::from_timestamp_nanos(log_record.time_unix_nano as i64)
                    } else {
                        Utc::now()
                    };

                    let severity_number = log_record.severity_number;
                    let severity_text = log_record.severity_text;

                    // Convert OTLP severity to our log level
                    let level = match severity_number {
                        1..=4 => bridge_core::types::LogLevel::Trace,
                        5..=8 => bridge_core::types::LogLevel::Debug,
                        9..=12 => bridge_core::types::LogLevel::Info,
                        13..=16 => bridge_core::types::LogLevel::Warn,
                        17..=20 => bridge_core::types::LogLevel::Error,
                        21..=24 => bridge_core::types::LogLevel::Fatal,
                        _ => bridge_core::types::LogLevel::Info,
                    };

                    let message = log_record
                        .body
                        .map(|v| match v.value {
                            Some(AnyValueValue::StringValue(s)) => s,
                            Some(AnyValueValue::IntValue(i)) => i.to_string(),
                            Some(AnyValueValue::DoubleValue(d)) => d.to_string(),
                            Some(AnyValueValue::BoolValue(b)) => b.to_string(),
                            _ => "unknown".to_string(),
                        })
                        .unwrap_or_default();

                    let attributes: HashMap<String, String> = log_record
                        .attributes
                        .into_iter()
                        .map(|attr| {
                            (
                                attr.key,
                                attr.value
                                    .map(|v| match v.value {
                                        Some(AnyValueValue::StringValue(s)) => s,
                                        Some(AnyValueValue::IntValue(i)) => i.to_string(),
                                        Some(AnyValueValue::DoubleValue(d)) => d.to_string(),
                                        Some(AnyValueValue::BoolValue(b)) => b.to_string(),
                                        _ => "unknown".to_string(),
                                    })
                                    .unwrap_or_default(),
                            )
                        })
                        .collect();

                    records.push(TelemetryRecord {
                        id: Uuid::new_v4(),
                        timestamp,
                        record_type: TelemetryType::Log,
                        data: TelemetryData::Log(LogData {
                            timestamp,
                            level,
                            message,
                            attributes: attributes.clone(),
                            body: None,
                            severity_number: Some(severity_number as u32),
                            severity_text: Some(severity_text),
                        }),
                        attributes: resource_attrs.clone(),
                        tags: HashMap::new(),
                        resource: None,
                        service: None,
                    });
                }
            }
        }

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "otlp-grpc".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::from([
                ("protocol".to_string(), "otlp-grpc".to_string()),
                ("content_type".to_string(), "application/grpc".to_string()),
            ]),
        })
    }
}

#[async_trait]
impl ProtocolHandler for OtlpGrpcProtocol {
    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing OTLP gRPC protocol handler");

        // Validate configuration
        self.config.validate().await?;

        // Initialize gRPC server
        self.init_server().await?;

        info!("OTLP gRPC protocol handler initialized");
        Ok(())
    }

    async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting OTLP gRPC protocol handler");

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

        info!("OTLP gRPC protocol handler started");
        Ok(())
    }

    async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping OTLP gRPC protocol handler");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = false;
        }

        info!("OTLP gRPC protocol handler stopped");
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
        // For OTLP gRPC, data is received through the gRPC service handlers
        // This method is not used in the gRPC flow since data comes through
        // the service export methods directly

        // Update stats to indicate we're ready to receive data
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = true;
            stats.last_message_time = Some(Utc::now());
        }

        // Return None since gRPC data is handled asynchronously through service calls
        Ok(None)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// OTLP gRPC message handler
pub struct OtlpGrpcMessageHandler;

impl OtlpGrpcMessageHandler {
    pub fn new() -> Self {
        Self
    }

    /// Decode OTLP gRPC message
    async fn decode_otlp_message(&self, payload: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        // Implement OTLP message decoding
        // Decode the protobuf message and convert to TelemetryRecord

        // Try to decode as ExportMetricsServiceRequest first
        if let Ok(request) = ExportMetricsServiceRequest::decode(payload) {
            // Create a simple batch from the request
            let mut records = Vec::new();
            // For now, create a placeholder record since we can't call process_metrics_request
            records.push(TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name: "decoded_metric".to_string(),
                    description: Some("Decoded from OTLP gRPC".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: bridge_core::types::MetricType::Gauge,
                    value: MetricValue::Gauge(1.0),
                    labels: HashMap::new(),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::new(),
                tags: HashMap::new(),
                resource: None,
                service: None,
            });
            return Ok(records);
        }

        // Try to decode as ExportLogsServiceRequest
        if let Ok(request) = ExportLogsServiceRequest::decode(payload) {
            // Create a simple batch from the request
            let mut records = Vec::new();
            // For now, create a placeholder record since we can't call process_logs_request
            records.push(TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Log,
                data: TelemetryData::Log(LogData {
                    timestamp: Utc::now(),
                    level: bridge_core::types::LogLevel::Info,
                    message: "Decoded from OTLP gRPC".to_string(),
                    attributes: HashMap::new(),
                    body: None,
                    severity_number: Some(9),
                    severity_text: Some("INFO".to_string()),
                }),
                attributes: HashMap::new(),
                tags: HashMap::new(),
                resource: None,
                service: None,
            });
            return Ok(records);
        }

        // Try to decode as ExportTraceServiceRequest (if traces are enabled)
        // if let Ok(request) = ExportTraceServiceRequest::decode(payload) {
        //     let batch = self.process_traces_request(request).await?;
        //     return Ok(batch.records);
        // }

        // If none of the above work, try to decode as a generic OTLP message
        // This is a fallback for unknown message types
        info!("Unknown OTLP message type, creating generic record");

        Ok(vec![TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp: Utc::now(),
                level: bridge_core::types::LogLevel::Warn,
                message: "Unknown OTLP message type".to_string(),
                attributes: HashMap::from([
                    ("protocol".to_string(), "otlp-grpc".to_string()),
                    ("message_size".to_string(), payload.len().to_string()),
                ]),
                body: None,
                severity_number: Some(13),
                severity_text: Some("warn".to_string()),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        }])
    }
}

#[async_trait]
impl MessageHandler for OtlpGrpcMessageHandler {
    async fn handle_message(&self, message: ProtocolMessage) -> BridgeResult<TelemetryBatch> {
        let records = match &message.payload {
            super::MessagePayload::Protobuf(data) => self.decode_otlp_message(data).await?,
            super::MessagePayload::Json(data) => self.decode_otlp_message(data).await?,
            super::MessagePayload::Arrow(data) => self.decode_otlp_message(data).await?,
        };

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: message.timestamp,
            source: message.protocol,
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

/// OTLP gRPC metrics service implementation
#[derive(Clone)]
pub struct OtlpMetricsService {
    protocol: Arc<OtlpGrpcProtocol>,
}

#[tonic::async_trait]
impl MetricsService for OtlpMetricsService {
    async fn export(
        &self,
        request: tonic::Request<ExportMetricsServiceRequest>,
    ) -> Result<tonic::Response<ExportMetricsServiceResponse>, Status> {
        let request_data = request.into_inner();

        match self.protocol.process_metrics_request(request_data).await {
            Ok(_batch) => {
                let response = ExportMetricsServiceResponse {
                    partial_success: None,
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("Failed to process metrics request: {}", e);
                Err(Status::internal("Failed to process metrics"))
            }
        }
    }
}

/// OTLP gRPC logs service implementation
#[derive(Clone)]
pub struct OtlpLogsService {
    protocol: Arc<OtlpGrpcProtocol>,
}

#[tonic::async_trait]
impl LogsService for OtlpLogsService {
    async fn export(
        &self,
        request: tonic::Request<ExportLogsServiceRequest>,
    ) -> Result<tonic::Response<ExportLogsServiceResponse>, Status> {
        let request_data = request.into_inner();

        match self.protocol.process_logs_request(request_data).await {
            Ok(_batch) => {
                let response = ExportLogsServiceResponse {
                    partial_success: None,
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("Failed to process logs request: {}", e);
                Err(Status::internal("Failed to process logs"))
            }
        }
    }
}
