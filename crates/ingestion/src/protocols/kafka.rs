//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Kafka protocol implementation
//!
//! This module provides support for ingesting OpenTelemetry data from Kafka topics
//! with support for various serialization formats.

use ::prost::Message;
use arrow_array::{Array, Float64Array, Int64Array, RecordBatch, StringArray};
use async_trait::async_trait;
use bridge_core::{
    types::{LogData, MetricData, MetricValue, TelemetryData, TelemetryRecord, TelemetryType},
    BridgeResult, TelemetryBatch,
};
use chrono::{DateTime, Utc};
use bridge_core::receivers::otlp::otlp::opentelemetry::proto::common::v1::any_value::Value as AnyValueValue;
use bridge_core::receivers::otlp::otlp::opentelemetry::proto::metrics::v1::metric::Data as OtlpMetricData;
use bridge_core::receivers::otlp::otlp::opentelemetry::proto::metrics::v1::number_data_point::Value as NumberDataPointValue;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

// Add rdkafka imports for Kafka consumer implementation
#[cfg(feature = "kafka")]
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    error::KafkaError,
    message::OwnedHeaders,
    types::RDKafkaErrorCode,
    ClientConfig, Message as KafkaMessage, Offset, TopicPartitionList,
};

use super::{MessageHandler, ProtocolConfig, ProtocolHandler, ProtocolMessage, ProtocolStats};

/// Kafka protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Protocol name
    pub name: String,

    /// Protocol version
    pub version: String,

    /// Kafka bootstrap servers
    pub bootstrap_servers: Vec<String>,

    /// Topic to consume from
    pub topic: String,

    /// Consumer group ID
    pub group_id: String,

    /// Auto offset reset policy
    pub auto_offset_reset: String,

    /// Enable auto commit
    pub enable_auto_commit: bool,

    /// Auto commit interval in milliseconds
    pub auto_commit_interval_ms: u64,

    /// Session timeout in milliseconds
    pub session_timeout_ms: u64,

    /// Max poll records
    pub max_poll_records: usize,

    /// Batch size for processing
    pub batch_size: usize,

    /// Buffer size for incoming data
    pub buffer_size: usize,

    /// Serialization format
    pub serialization_format: KafkaSerializationFormat,

    /// Security configuration
    pub security: Option<KafkaSecurityConfig>,

    /// Additional configuration properties
    pub properties: HashMap<String, String>,
}

/// Kafka serialization formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KafkaSerializationFormat {
    Json,
    Avro,
    Protobuf,
    Arrow,
    Parquet,
}

/// Kafka security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaSecurityConfig {
    /// SASL mechanism
    pub sasl_mechanism: Option<String>,

    /// SASL username
    pub sasl_username: Option<String>,

    /// SASL password
    pub sasl_password: Option<String>,

    /// SSL CA certificate path
    pub ssl_ca_cert_path: Option<String>,

    /// SSL client certificate path
    pub ssl_client_cert_path: Option<String>,

    /// SSL client key path
    pub ssl_client_key_path: Option<String>,
}

impl KafkaConfig {
    /// Create new Kafka configuration
    pub fn new(bootstrap_servers: Vec<String>, topic: String, group_id: String) -> Self {
        Self {
            name: "kafka".to_string(),
            version: "1.0.0".to_string(),
            bootstrap_servers,
            topic,
            group_id,
            auto_offset_reset: "earliest".to_string(),
            enable_auto_commit: true,
            auto_commit_interval_ms: 5000,
            session_timeout_ms: 30000,
            max_poll_records: 500,
            batch_size: 1000,
            buffer_size: 10000,
            serialization_format: KafkaSerializationFormat::Json,
            security: None,
            properties: HashMap::new(),
        }
    }
}

#[async_trait]
impl ProtocolConfig for KafkaConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.bootstrap_servers.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Kafka bootstrap servers cannot be empty",
            ));
        }

        if self.topic.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Kafka topic cannot be empty",
            ));
        }

        if self.group_id.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Kafka group ID cannot be empty",
            ));
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Kafka protocol handler
pub struct KafkaProtocol {
    config: KafkaConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ProtocolStats>>,
    message_handler: Arc<dyn MessageHandler>,
    consumer: Option<KafkaConsumer>,
}

/// Kafka consumer wrapper
struct KafkaConsumer {
    #[cfg(feature = "kafka")]
    consumer: Option<StreamConsumer>,
    #[cfg(not(feature = "kafka"))]
    _phantom: std::marker::PhantomData<()>,
    topic: String,
    is_running: bool,
}

impl KafkaConsumer {
    #[cfg(feature = "kafka")]
    async fn new(config: &KafkaConfig) -> BridgeResult<Self> {
        let mut client_config = ClientConfig::new();

        // Set basic configuration
        client_config.set("bootstrap.servers", &config.bootstrap_servers.join(","));
        client_config.set("group.id", &config.group_id);
        client_config.set("auto.offset.reset", &config.auto_offset_reset);
        client_config.set("enable.auto.commit", &config.enable_auto_commit.to_string());
        client_config.set(
            "auto.commit.interval.ms",
            &config.auto_commit_interval_ms.to_string(),
        );
        client_config.set("session.timeout.ms", &config.session_timeout_ms.to_string());
        client_config.set("max.poll.records", &config.max_poll_records.to_string());

        // Set security configuration if provided
        if let Some(security) = &config.security {
            if let Some(sasl_mechanism) = &security.sasl_mechanism {
                client_config.set("security.protocol", "SASL_PLAINTEXT");
                client_config.set("sasl.mechanism", sasl_mechanism);
            }

            if let Some(username) = &security.sasl_username {
                client_config.set("sasl.username", username);
            }

            if let Some(password) = &security.sasl_password {
                client_config.set("sasl.password", password);
            }

            if security.ssl_ca_cert_path.is_some() || security.ssl_client_cert_path.is_some() {
                client_config.set("security.protocol", "SSL");

                if let Some(ca_cert_path) = &security.ssl_ca_cert_path {
                    client_config.set("ssl.ca.location", ca_cert_path);
                }

                if let Some(client_cert_path) = &security.ssl_client_cert_path {
                    client_config.set("ssl.certificate.location", client_cert_path);
                }

                if let Some(client_key_path) = &security.ssl_client_key_path {
                    client_config.set("ssl.key.location", client_key_path);
                }
            }
        }

        // Set additional properties
        for (key, value) in &config.properties {
            client_config.set(key, value);
        }

        // Create the consumer
        let consumer: StreamConsumer = client_config.create().map_err(|e| {
            bridge_core::BridgeError::network(format!("Failed to create Kafka consumer: {}", e))
        })?;

        // Subscribe to the topic
        consumer.subscribe(&[&config.topic]).map_err(|e| {
            bridge_core::BridgeError::network(format!(
                "Failed to subscribe to Kafka topic {}: {}",
                config.topic, e
            ))
        })?;

        Ok(Self {
            consumer: Some(consumer),
            topic: config.topic.clone(),
            is_running: false,
        })
    }

    #[cfg(not(feature = "kafka"))]
    async fn new(_config: &KafkaConfig) -> BridgeResult<Self> {
        Err(bridge_core::BridgeError::configuration(
            "Kafka feature is not enabled. Enable the 'kafka' feature to use Kafka functionality."
                .to_string(),
        ))
    }

    #[cfg(feature = "kafka")]
    async fn poll(&mut self) -> BridgeResult<Vec<(Option<Vec<u8>>, Vec<u8>)>> {
        let consumer = self.consumer.as_ref().ok_or_else(|| {
            bridge_core::BridgeError::network("Kafka consumer not initialized".to_string())
        })?;

        let mut messages = Vec::new();

        // Poll for messages with a timeout
        match tokio::time::timeout(std::time::Duration::from_millis(100), consumer.recv()).await {
            Ok(Ok(message)) => {
                let key = message.key().map(|k| k.to_vec());
                let value = message.payload().map(|p| p.to_vec()).unwrap_or_default();

                messages.push((key, value));

                // Commit the offset
                if let Err(e) =
                    consumer.commit_message(&message, rdkafka::consumer::CommitMode::Async)
                {
                    warn!("Failed to commit Kafka message offset: {}", e);
                }
            }
            Ok(Err(KafkaError::MessageConsumption(RDKafkaErrorCode::TimedOut))) => {
                // Timeout is expected, no messages available
            }
            Ok(Err(e)) => {
                return Err(bridge_core::BridgeError::network(format!(
                    "Kafka message consumption error: {}",
                    e
                )));
            }
            Err(_) => {
                // Timeout occurred, no messages available
            }
        }

        Ok(messages)
    }

    #[cfg(not(feature = "kafka"))]
    async fn poll(&mut self) -> BridgeResult<Vec<(Option<Vec<u8>>, Vec<u8>)>> {
        Err(bridge_core::BridgeError::configuration(
            "Kafka feature is not enabled".to_string(),
        ))
    }

    fn is_running(&self) -> bool {
        self.is_running
    }

    fn stop(&mut self) {
        self.is_running = false;
    }
}

impl KafkaProtocol {
    /// Create new Kafka protocol handler
    pub async fn new(config: &dyn ProtocolConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<KafkaConfig>()
            .ok_or_else(|| bridge_core::BridgeError::configuration("Invalid Kafka configuration"))?
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
            message_handler: Arc::new(KafkaMessageHandler::new()),
            consumer: None,
        })
    }

    /// Initialize Kafka consumer
    async fn init_consumer(&mut self) -> BridgeResult<()> {
        info!(
            "Initializing Kafka consumer for topic: {}",
            self.config.topic
        );

        // Create Kafka consumer
        let consumer = KafkaConsumer::new(&self.config).await?;
        self.consumer = Some(consumer);

        info!("Kafka consumer initialized");
        Ok(())
    }

    /// Start consuming messages from Kafka
    async fn start_consuming(&self) -> BridgeResult<()> {
        info!("Starting Kafka message consumption");

        let config = self.config.clone();
        let is_running = self.is_running.clone();
        let stats = self.stats.clone();
        let message_handler = self.message_handler.clone();
        let topic = self.config.topic.clone();

        // Spawn a background task for message consumption
        tokio::spawn(async move {
            info!(
                "Kafka message consumption loop started for topic: {}",
                topic
            );

            let mut consumer = match KafkaConsumer::new(&config).await {
                Ok(consumer) => consumer,
                Err(e) => {
                    error!("Failed to create Kafka consumer: {}", e);
                    return;
                }
            };

            let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));

            loop {
                interval.tick().await;

                // Check if we should continue running
                {
                    let running = is_running.read().await;
                    if !*running {
                        break;
                    }
                }

                // Poll for messages
                match consumer.poll().await {
                    Ok(messages) => {
                        for (key, value) in messages {
                            let value_len = value.len();

                            // Process the message
                            let result = Self::process_kafka_message_internal(
                                &config,
                                &message_handler,
                                key,
                                value,
                            )
                            .await;

                            // Update stats
                            let mut stats = stats.write().await;
                            stats.total_messages += 1;
                            stats.total_bytes += value_len as u64;
                            stats.last_message_time = Some(Utc::now());

                            if let Err(e) = result {
                                stats.error_count += 1;
                                error!("Failed to process Kafka message: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        let mut stats = stats.write().await;
                        stats.error_count += 1;
                        error!("Kafka polling error: {}", e);
                    }
                }
            }

            consumer.stop();
            info!(
                "Kafka message consumption loop stopped for topic: {}",
                topic
            );
        });

        Ok(())
    }

    /// Internal method to process Kafka message
    async fn process_kafka_message_internal(
        config: &KafkaConfig,
        message_handler: &Arc<dyn MessageHandler>,
        key: Option<Vec<u8>>,
        value: Vec<u8>,
    ) -> BridgeResult<TelemetryBatch> {
        let message = ProtocolMessage {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            protocol: "kafka".to_string(),
            payload: super::MessagePayload::Json(value),
            metadata: {
                let mut metadata = HashMap::new();
                if let Some(key) = key {
                    metadata.insert(
                        "kafka_key".to_string(),
                        String::from_utf8_lossy(&key).to_string(),
                    );
                }
                metadata.insert("topic".to_string(), config.topic.clone());
                metadata.insert("group_id".to_string(), config.group_id.clone());
                metadata
            },
        };

        message_handler.handle_message(message).await
    }

    /// Process Kafka message
    async fn process_kafka_message(
        &self,
        key: Option<Vec<u8>>,
        value: Vec<u8>,
    ) -> BridgeResult<TelemetryBatch> {
        let message = ProtocolMessage {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            protocol: "kafka".to_string(),
            payload: super::MessagePayload::Json(value),
            metadata: {
                let mut metadata = HashMap::new();
                if let Some(key) = key {
                    metadata.insert(
                        "kafka_key".to_string(),
                        String::from_utf8_lossy(&key).to_string(),
                    );
                }
                metadata.insert("topic".to_string(), self.config.topic.clone());
                metadata.insert("group_id".to_string(), self.config.group_id.clone());
                metadata
            },
        };

        self.message_handler.handle_message(message).await
    }
}

#[async_trait]
impl ProtocolHandler for KafkaProtocol {
    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing Kafka protocol handler");

        // Validate configuration
        self.config.validate().await?;

        // Initialize Kafka consumer
        self.init_consumer().await?;

        info!("Kafka protocol handler initialized");
        Ok(())
    }

    async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting Kafka protocol handler");

        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = true;
        }

        // Start consuming messages
        self.start_consuming().await?;

        info!("Kafka protocol handler started");
        Ok(())
    }

    async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping Kafka protocol handler");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = false;
        }

        info!("Kafka protocol handler stopped");
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
        // Try to get a message from the consumer if available
        if let Some(consumer) = &self.consumer {
            #[cfg(feature = "kafka")]
            {
                // This is a simplified implementation - in practice, the consumer
                // would be polled in the background task and messages would be
                // queued for processing
                return Ok(None);
            }

            #[cfg(not(feature = "kafka"))]
            {
                return Err(bridge_core::BridgeError::configuration(
                    "Kafka feature is not enabled".to_string(),
                ));
            }
        }

        Ok(None)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Kafka message handler
pub struct KafkaMessageHandler {
    serialization_format: KafkaSerializationFormat,
}

impl KafkaMessageHandler {
    pub fn new() -> Self {
        Self {
            serialization_format: KafkaSerializationFormat::Json,
        }
    }

    /// Decode Kafka message based on serialization format
    async fn decode_kafka_message(&self, payload: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        match self.serialization_format {
            KafkaSerializationFormat::Json => self.decode_json_message(payload).await,
            KafkaSerializationFormat::Avro => self.decode_avro_message(payload).await,
            KafkaSerializationFormat::Protobuf => self.decode_protobuf_message(payload).await,
            KafkaSerializationFormat::Arrow => self.decode_arrow_message(payload).await,
            KafkaSerializationFormat::Parquet => self.decode_parquet_message(payload).await,
        }
    }

    /// Decode JSON message
    async fn decode_json_message(&self, payload: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        // Implement JSON decoding
        // Parse JSON and convert to TelemetryRecord

        let json_str = std::str::from_utf8(payload).map_err(|e| {
            bridge_core::BridgeError::serialization(format!(
                "Failed to decode payload as UTF-8: {}",
                e
            ))
        })?;

        // Try to parse as a single record first
        if let Ok(record) = serde_json::from_str::<TelemetryRecord>(json_str) {
            return Ok(vec![record]);
        }

        // Try to parse as an array of records
        if let Ok(records) = serde_json::from_str::<Vec<TelemetryRecord>>(json_str) {
            return Ok(records);
        }

        // Try to parse as a TelemetryBatch
        if let Ok(batch) = serde_json::from_str::<TelemetryBatch>(json_str) {
            return Ok(batch.records);
        }

        // Try to parse as a generic JSON object and create a log record
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str) {
            let message = if let Some(msg) = json_value.get("message").and_then(|v| v.as_str()) {
                msg.to_string()
            } else {
                format!("Kafka JSON message: {}", json_value)
            };

            let level = if let Some(level_str) = json_value.get("level").and_then(|v| v.as_str()) {
                match level_str.to_lowercase().as_str() {
                    "trace" => bridge_core::types::LogLevel::Trace,
                    "debug" => bridge_core::types::LogLevel::Debug,
                    "info" => bridge_core::types::LogLevel::Info,
                    "warn" | "warning" => bridge_core::types::LogLevel::Warn,
                    "error" => bridge_core::types::LogLevel::Error,
                    "fatal" => bridge_core::types::LogLevel::Fatal,
                    _ => bridge_core::types::LogLevel::Info,
                }
            } else {
                bridge_core::types::LogLevel::Info
            };

            let attributes: HashMap<String, String> = if let Some(obj) = json_value.as_object() {
                obj.iter()
                    .filter_map(|(k, v)| {
                        if k != "message" && k != "level" {
                            Some((k.clone(), v.to_string()))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                HashMap::new()
            };

            return Ok(vec![TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Log,
                data: TelemetryData::Log(LogData {
                    timestamp: Utc::now(),
                    level,
                    message,
                    attributes,
                    body: None,
                    severity_number: None,
                    severity_text: None,
                }),
                attributes: HashMap::new(),
                tags: HashMap::new(),
                resource: None,
                service: None,
            }]);
        }

        // If all parsing attempts fail, create an error log record
        Ok(vec![TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp: Utc::now(),
                level: bridge_core::types::LogLevel::Error,
                message: format!("Failed to parse Kafka JSON message: {}", json_str),
                attributes: HashMap::from([
                    ("protocol".to_string(), "kafka".to_string()),
                    ("serialization".to_string(), "json".to_string()),
                    ("payload_size".to_string(), payload.len().to_string()),
                ]),
                body: None,
                severity_number: None,
                severity_text: None,
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        }])
    }

    /// Decode Avro message
    async fn decode_avro_message(&self, payload: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        // Implement Avro decoding using avro-rs
        use avro_rs::Reader;

        // For now, we'll create a simple implementation that attempts to decode
        // Avro data and convert it to telemetry records
        // In a real implementation, you would need the Avro schema to properly decode

        // Try to create a reader from the payload
        match Reader::new(payload) {
            Ok(reader) => {
                let mut records = Vec::new();

                // Read records from the Avro reader
                for value in reader {
                    match value {
                        Ok(avro_value) => {
                            // Convert Avro value to JSON for easier processing
                            let json_value = self.avro_value_to_json(avro_value);

                            // Create a telemetry record from the JSON value
                            let message = if let Some(msg) =
                                json_value.get("message").and_then(|v| v.as_str())
                            {
                                msg.to_string()
                            } else {
                                format!("Avro message: {}", json_value)
                            };

                            let level = if let Some(level_str) =
                                json_value.get("level").and_then(|v| v.as_str())
                            {
                                match level_str.to_lowercase().as_str() {
                                    "trace" => bridge_core::types::LogLevel::Trace,
                                    "debug" => bridge_core::types::LogLevel::Debug,
                                    "info" => bridge_core::types::LogLevel::Info,
                                    "warn" | "warning" => bridge_core::types::LogLevel::Warn,
                                    "error" => bridge_core::types::LogLevel::Error,
                                    "fatal" => bridge_core::types::LogLevel::Fatal,
                                    _ => bridge_core::types::LogLevel::Info,
                                }
                            } else {
                                bridge_core::types::LogLevel::Info
                            };

                            let attributes: HashMap<String, String> =
                                if let Some(obj) = json_value.as_object() {
                                    obj.iter()
                                        .filter_map(|(k, v)| {
                                            if k != "message" && k != "level" {
                                                Some((k.clone(), v.to_string()))
                                            } else {
                                                None
                                            }
                                        })
                                        .collect()
                                } else {
                                    HashMap::new()
                                };

                            records.push(TelemetryRecord {
                                id: Uuid::new_v4(),
                                timestamp: Utc::now(),
                                record_type: TelemetryType::Log,
                                data: TelemetryData::Log(LogData {
                                    timestamp: Utc::now(),
                                    level,
                                    message,
                                    attributes,
                                    body: None,
                                    severity_number: None,
                                    severity_text: None,
                                }),
                                attributes: HashMap::from([
                                    ("protocol".to_string(), "kafka".to_string()),
                                    ("serialization".to_string(), "avro".to_string()),
                                    ("payload_size".to_string(), payload.len().to_string()),
                                ]),
                                tags: HashMap::new(),
                                resource: None,
                                service: None,
                            });
                        }
                        Err(e) => {
                            warn!("Failed to decode Avro record: {}", e);
                        }
                    }
                }

                if !records.is_empty() {
                    return Ok(records);
                }
            }
            Err(e) => {
                warn!("Failed to create Avro reader: {}", e);
            }
        }

        // If Avro decoding fails, create an error log record
        Ok(vec![TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp: Utc::now(),
                level: bridge_core::types::LogLevel::Error,
                message: "Failed to decode Kafka Avro message".to_string(),
                attributes: HashMap::from([
                    ("protocol".to_string(), "kafka".to_string()),
                    ("serialization".to_string(), "avro".to_string()),
                    ("payload_size".to_string(), payload.len().to_string()),
                ]),
                body: None,
                severity_number: None,
                severity_text: None,
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        }])
    }

    /// Decode Protobuf message
    async fn decode_protobuf_message(&self, payload: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        // Implement Protobuf decoding
        // Use prost to decode the payload

        // Try to decode as OTLP ExportMetricsServiceRequest
        if let Ok(request) =
            bridge_core::receivers::otlp::otlp::opentelemetry::proto::collector::metrics::v1::ExportMetricsServiceRequest::decode(
                payload,
            )
        {
            // Convert OTLP metrics to our format
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

                        while let Some(ref data_point) = metric.data {
                            match data_point {
                                OtlpMetricData::Gauge(gauge) => {
                                    for point in gauge.data_points.clone() {
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
                                                            Some(AnyValueValue::StringValue(s)) => {
                                                                s
                                                            }
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
                                            DateTime::from_timestamp_nanos(
                                                point.time_unix_nano as i64,
                                            )
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
                                _ => {
                                    // Handle other metric types as needed
                                    info!(
                                        "Unsupported Kafka protobuf metric type for metric: {}",
                                        metric_name
                                    );
                                }
                            }
                        }
                    }
                }
            }

            return Ok(records);
        }

        // Try to decode as OTLP ExportLogsServiceRequest
        if let Ok(request) =
            bridge_core::receivers::otlp::otlp::opentelemetry::proto::collector::logs::v1::ExportLogsServiceRequest::decode(
                payload,
            )
        {
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

            return Ok(records);
        }

        // If protobuf decoding fails, create an error log record
        Ok(vec![TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp: Utc::now(),
                level: bridge_core::types::LogLevel::Error,
                message: "Failed to decode Kafka protobuf message".to_string(),
                attributes: HashMap::from([
                    ("protocol".to_string(), "kafka".to_string()),
                    ("serialization".to_string(), "protobuf".to_string()),
                    ("payload_size".to_string(), payload.len().to_string()),
                ]),
                body: None,
                severity_number: None,
                severity_text: None,
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        }])
    }

    /// Decode Arrow message
    async fn decode_arrow_message(&self, payload: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        // Implement Arrow decoding using arrow-ipc
        use arrow_ipc::reader::StreamReader;

        // Try to create a stream reader from the payload
        match StreamReader::try_new(std::io::Cursor::new(payload), None) {
            Ok(mut reader) => {
                let mut records = Vec::new();

                // Read record batches from the Arrow stream
                while let Some(batch_result) = reader.next() {
                    match batch_result {
                        Ok(batch) => {
                            // Process the record batch
                            let batch_records = self.process_arrow_batch(&batch).await?;
                            records.extend(batch_records);
                        }
                        Err(e) => {
                            warn!("Failed to read Arrow record batch: {}", e);
                        }
                    }
                }

                if !records.is_empty() {
                    return Ok(records);
                }
            }
            Err(e) => {
                warn!("Failed to create Arrow stream reader: {}", e);
            }
        }

        // If Arrow decoding fails, create an error log record
        Ok(vec![TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp: Utc::now(),
                level: bridge_core::types::LogLevel::Error,
                message: "Failed to decode Kafka Arrow message".to_string(),
                attributes: HashMap::from([
                    ("protocol".to_string(), "kafka".to_string()),
                    ("serialization".to_string(), "arrow".to_string()),
                    ("payload_size".to_string(), payload.len().to_string()),
                ]),
                body: None,
                severity_number: None,
                severity_text: None,
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        }])
    }

    /// Process Arrow record batch
    async fn process_arrow_batch(&self, batch: &RecordBatch) -> BridgeResult<Vec<TelemetryRecord>> {
        let mut records = Vec::new();

        // Get the number of rows in the batch
        let num_rows = batch.num_rows();

        // Try to extract common telemetry fields
        let timestamp_col = batch.column_by_name("timestamp");
        let name_col = batch.column_by_name("name");
        let value_col = batch.column_by_name("value");
        let message_col = batch.column_by_name("message");
        let level_col = batch.column_by_name("level");

        for row_idx in 0..num_rows {
            let timestamp = if let Some(ts_col) = timestamp_col {
                // Try to extract timestamp from various types
                if let Some(ts_array) = ts_col.as_any().downcast_ref::<Int64Array>() {
                    let ts_value = ts_array.value(row_idx);
                    DateTime::from_timestamp_millis(ts_value).unwrap_or_else(|| Utc::now())
                } else {
                    Utc::now()
                }
            } else {
                Utc::now()
            };

            // Try to create a metric record
            if let (Some(name_array), Some(value_array)) = (
                name_col.and_then(|col| col.as_any().downcast_ref::<StringArray>()),
                value_col.and_then(|col| col.as_any().downcast_ref::<Float64Array>()),
            ) {
                if let (Some(name), Some(value)) = (
                    Some(name_array.value(row_idx).to_string()),
                    Some(value_array.value(row_idx)),
                ) {
                    records.push(TelemetryRecord {
                        id: Uuid::new_v4(),
                        timestamp,
                        record_type: TelemetryType::Metric,
                        data: TelemetryData::Metric(MetricData {
                            name: name.to_string(),
                            description: Some("Decoded from Arrow format".to_string()),
                            unit: Some("count".to_string()),
                            metric_type: bridge_core::types::MetricType::Gauge,
                            value: MetricValue::Gauge(value),
                            labels: HashMap::from([
                                ("source".to_string(), "arrow_decoder".to_string()),
                                ("row_index".to_string(), row_idx.to_string()),
                            ]),
                            timestamp,
                        }),
                        attributes: HashMap::from([
                            ("protocol".to_string(), "kafka".to_string()),
                            ("serialization".to_string(), "arrow".to_string()),
                        ]),
                        tags: HashMap::new(),
                        resource: None,
                        service: None,
                    });
                    continue;
                }
            }

            // Try to create a log record
            if let Some(message_array) =
                message_col.and_then(|col| col.as_any().downcast_ref::<StringArray>())
            {
                let message = message_array.value(row_idx);
                let level = if let Some(level_array) =
                    level_col.and_then(|col| col.as_any().downcast_ref::<StringArray>())
                {
                    let level_str = level_array.value(row_idx);
                    match level_str.to_lowercase().as_str() {
                        "trace" => bridge_core::types::LogLevel::Trace,
                        "debug" => bridge_core::types::LogLevel::Debug,
                        "info" => bridge_core::types::LogLevel::Info,
                        "warn" | "warning" => bridge_core::types::LogLevel::Warn,
                        "error" => bridge_core::types::LogLevel::Error,
                        "fatal" => bridge_core::types::LogLevel::Fatal,
                        _ => bridge_core::types::LogLevel::Info,
                    }
                } else {
                    bridge_core::types::LogLevel::Info
                };

                records.push(TelemetryRecord {
                    id: Uuid::new_v4(),
                    timestamp,
                    record_type: TelemetryType::Log,
                    data: TelemetryData::Log(LogData {
                        timestamp,
                        level,
                        message: message.to_string(),
                        attributes: HashMap::from([
                            ("source".to_string(), "arrow_decoder".to_string()),
                            ("row_index".to_string(), row_idx.to_string()),
                        ]),
                        body: None,
                        severity_number: None,
                        severity_text: None,
                    }),
                    attributes: HashMap::from([
                        ("protocol".to_string(), "kafka".to_string()),
                        ("serialization".to_string(), "arrow".to_string()),
                    ]),
                    tags: HashMap::new(),
                    resource: None,
                    service: None,
                });
            }
        }

        Ok(records)
    }

    /// Decode Parquet message
    async fn decode_parquet_message(&self, payload: &[u8]) -> BridgeResult<Vec<TelemetryRecord>> {
        // Implement Parquet decoding using parquet crate
        use parquet::file::reader::{FileReader, SerializedFileReader};

        // Try to create a parquet reader from the payload
        match SerializedFileReader::new(prost::bytes::Bytes::from(payload.to_vec())) {
            Ok(reader) => {
                let mut records = Vec::new();

                // Read record batches from the parquet file
                for row_group_idx in 0..reader.num_row_groups() {
                    if let Ok(row_group_reader) = reader.get_row_group(row_group_idx) {
                        // Convert row group to record batch
                        // Note: This is a simplified implementation - in a real scenario,
                        // you would need to properly convert the row group to Arrow format
                        // For now, we'll skip this and create a fallback record
                        let fallback_record = TelemetryRecord {
                            id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            record_type: TelemetryType::Log,
                            data: TelemetryData::Log(LogData {
                                timestamp: Utc::now(),
                                level: bridge_core::types::LogLevel::Info,
                                message: "Parquet row group processed".to_string(),
                                attributes: HashMap::from([
                                    ("protocol".to_string(), "kafka".to_string()),
                                    ("serialization".to_string(), "parquet".to_string()),
                                    ("row_group".to_string(), row_group_idx.to_string()),
                                ]),
                                body: None,
                                severity_number: None,
                                severity_text: None,
                            }),
                            attributes: HashMap::new(),
                            tags: HashMap::new(),
                            resource: None,
                            service: None,
                        };
                        records.push(fallback_record);
                    }
                }

                if !records.is_empty() {
                    return Ok(records);
                }
            }
            Err(e) => {
                warn!("Failed to create Parquet reader: {}", e);
            }
        }

        // If Parquet decoding fails, create an error log record
        Ok(vec![TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp: Utc::now(),
                level: bridge_core::types::LogLevel::Error,
                message: "Failed to decode Kafka Parquet message".to_string(),
                attributes: HashMap::from([
                    ("protocol".to_string(), "kafka".to_string()),
                    ("serialization".to_string(), "parquet".to_string()),
                    ("payload_size".to_string(), payload.len().to_string()),
                ]),
                body: None,
                severity_number: None,
                severity_text: None,
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        }])
    }

    /// Convert Avro value to JSON value
    fn avro_value_to_json(&self, avro_value: avro_rs::types::Value) -> JsonValue {
        match avro_value {
            avro_rs::types::Value::Null => JsonValue::Null,
            avro_rs::types::Value::Boolean(b) => JsonValue::Bool(b),
            avro_rs::types::Value::Int(i) => JsonValue::Number(serde_json::Number::from(i)),
            avro_rs::types::Value::Long(l) => JsonValue::Number(serde_json::Number::from(l)),
            avro_rs::types::Value::Float(f) => {
                if let Some(n) = serde_json::Number::from_f64(f as f64) {
                    JsonValue::Number(n)
                } else {
                    JsonValue::Null
                }
            }
            avro_rs::types::Value::Double(d) => {
                if let Some(n) = serde_json::Number::from_f64(d) {
                    JsonValue::Number(n)
                } else {
                    JsonValue::Null
                }
            }
            avro_rs::types::Value::Bytes(b) => {
                JsonValue::String(String::from_utf8_lossy(&b).to_string())
            }
            avro_rs::types::Value::String(s) => JsonValue::String(s),
            avro_rs::types::Value::Array(arr) => {
                let json_array: Vec<JsonValue> = arr
                    .into_iter()
                    .map(|v| self.avro_value_to_json(v))
                    .collect();
                JsonValue::Array(json_array)
            }
            avro_rs::types::Value::Map(map) => {
                let json_map: serde_json::Map<String, JsonValue> = map
                    .into_iter()
                    .map(|(k, v)| (k, self.avro_value_to_json(v)))
                    .collect();
                JsonValue::Object(json_map)
            }
            avro_rs::types::Value::Union(union) => {
                // For union types, we'll use the first non-null value
                match *union {
                    avro_rs::types::Value::Union(inner) => self.avro_value_to_json(*inner),
                    _ => JsonValue::Null,
                }
            }
            avro_rs::types::Value::Fixed(_, bytes) => {
                JsonValue::String(String::from_utf8_lossy(&bytes).to_string())
            }
            avro_rs::types::Value::Enum(_, symbol) => JsonValue::String(symbol),
            avro_rs::types::Value::Record(fields) => {
                let json_map: serde_json::Map<String, JsonValue> = fields
                    .into_iter()
                    .map(|(k, v)| (k, self.avro_value_to_json(v)))
                    .collect();
                JsonValue::Object(json_map)
            }
            avro_rs::types::Value::Date(_) => JsonValue::Null,
            avro_rs::types::Value::Decimal(_) => JsonValue::Null,
            avro_rs::types::Value::TimeMillis(_) => JsonValue::Null,
            avro_rs::types::Value::TimeMicros(_) => JsonValue::Null,
            avro_rs::types::Value::TimestampMillis(_) => JsonValue::Null,
            avro_rs::types::Value::TimestampMicros(_) => JsonValue::Null,
            avro_rs::types::Value::Duration(_) => JsonValue::Null,
            avro_rs::types::Value::Uuid(_) => JsonValue::Null,
        }
    }
}

#[async_trait]
impl MessageHandler for KafkaMessageHandler {
    async fn handle_message(&self, message: ProtocolMessage) -> BridgeResult<TelemetryBatch> {
        let records = match &message.payload {
            super::MessagePayload::Json(data) => self.decode_kafka_message(data).await?,
            super::MessagePayload::Arrow(data) => self.decode_kafka_message(data).await?,
            super::MessagePayload::Protobuf(data) => self.decode_kafka_message(data).await?,
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
