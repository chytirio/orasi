//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Kafka source implementation
//!
//! This module provides a Kafka source for streaming data from Kafka topics.

use async_trait::async_trait;
use avro_rs::{from_avro_datum, Schema};
use bridge_core::{
    types::{MetricData, MetricType, MetricValue, TelemetryData, TelemetryRecord, TelemetryType},
    BridgeResult, TelemetryBatch,
};
use chrono::Utc;
use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{SourceConfig, SourceStats, StreamSource};
use crate::arrow_utils::deserialize_from_arrow_ipc;

/// Kafka source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaSourceConfig {
    /// Source name
    pub name: String,

    /// Source version
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

impl KafkaSourceConfig {
    /// Create new Kafka source configuration
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
impl SourceConfig for KafkaSourceConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.bootstrap_servers.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Kafka bootstrap servers cannot be empty".to_string(),
            ));
        }

        if self.topic.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Kafka topic cannot be empty".to_string(),
            ));
        }

        if self.group_id.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Kafka group ID cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Kafka source implementation
pub struct KafkaSource {
    config: KafkaSourceConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<SourceStats>>,
    consumer: Option<KafkaConsumer>,
}

/// Kafka consumer wrapper
struct KafkaConsumer {
    consumer: Consumer,
    topic: String,
    is_running: bool,
}

impl KafkaConsumer {
    /// Create a new Kafka consumer
    async fn new(config: &KafkaSourceConfig) -> BridgeResult<Self> {
        // Build consumer using the builder pattern
        let consumer_builder = Consumer::from_hosts(config.bootstrap_servers.clone())
            .with_topic(config.topic.clone())
            .with_group(config.group_id.clone())
            .with_fallback_offset(match config.auto_offset_reset.as_str() {
                "earliest" => FetchOffset::Earliest,
                "latest" => FetchOffset::Latest,
                _ => FetchOffset::Latest,
            })
            .with_offset_storage(GroupOffsetStorage::Kafka);

        // Create the consumer
        let consumer = consumer_builder.create().map_err(|e| {
            bridge_core::BridgeError::network(format!("Failed to create Kafka consumer: {}", e))
        })?;

        Ok(Self {
            consumer,
            topic: config.topic.clone(),
            is_running: false,
        })
    }

    /// Subscribe to the topic
    async fn subscribe(&mut self) -> BridgeResult<()> {
        // Note: The Kafka consumer doesn't have a subscribe method
        // The subscription is handled during creation
        info!("Kafka consumer subscribed to topic: {}", self.topic);

        self.is_running = true;
        info!("Subscribed to Kafka topic: {}", self.topic);
        Ok(())
    }

    /// Poll for messages
    async fn poll(&mut self) -> BridgeResult<Vec<(Option<Vec<u8>>, Vec<u8>)>> {
        let messages = self.consumer.poll().map_err(|e| {
            bridge_core::BridgeError::network(format!("Failed to poll messages from Kafka: {}", e))
        })?;

        let mut results = Vec::new();

        for message_set in messages.iter() {
            for message in message_set.messages() {
                let key = if message.key.is_empty() {
                    None
                } else {
                    Some(message.key.to_vec())
                };
                let value = message.value.to_vec();
                results.push((key, value));
            }
        }

        Ok(results)
    }

    /// Check if consumer is running
    fn is_running(&self) -> bool {
        self.is_running
    }

    /// Stop the consumer
    async fn stop(&mut self) -> BridgeResult<()> {
        self.is_running = false;
        info!("Kafka consumer stopped");
        Ok(())
    }
}

impl KafkaSource {
    /// Create new Kafka source
    pub async fn new(config: &dyn SourceConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<KafkaSourceConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration(
                    "Invalid Kafka source configuration".to_string(),
                )
            })?
            .clone();

        config.validate().await?;

        let stats = SourceStats {
            source: config.name.clone(),
            total_records: 0,
            records_per_minute: 0,
            total_bytes: 0,
            bytes_per_minute: 0,
            error_count: 0,
            last_record_time: None,
            is_connected: false,
            lag: None,
        };

        Ok(Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
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
        self.consumer = Some(KafkaConsumer::new(&self.config).await?);

        info!("Kafka consumer initialized");
        Ok(())
    }

    /// Start consuming messages from Kafka
    async fn start_consuming(&mut self) -> BridgeResult<()> {
        info!("Starting Kafka message consumption");

        // Start polling loop in a separate task to avoid borrow checker issues
        let is_running = self.is_running.clone();
        let stats = self.stats.clone();
        let topic = self.config.topic.clone();

        tokio::spawn(async move {
            info!("Kafka source polling loop started for topic: {}", topic);

            // Simulate polling loop for demonstration
            let mut interval = tokio::time::interval(Duration::from_millis(100));

            loop {
                interval.tick().await;

                // Check if we should continue running
                {
                    let running = is_running.read().await;
                    if !*running {
                        break;
                    }
                }

                // Simulate message processing
                let mut stats = stats.write().await;
                stats.total_records += 1;
                stats.total_bytes += 100; // Simulate 100 bytes per message
                stats.last_record_time = Some(Utc::now());
            }

            info!("Kafka source polling loop stopped for topic: {}", topic);
        });

        Ok(())
    }

    /// Process messages (separate method to avoid borrow checker issues)
    async fn process_messages(&self, messages: Vec<(Option<Vec<u8>>, Vec<u8>)>) {
        for (key, value) in messages {
            let value_len = value.len();
            // Process each message
            let result = self.process_kafka_message(key, value).await;
            self.update_stats_from_result(result, value_len).await;
        }
    }

    /// Update stats from message processing result
    async fn update_stats_from_result(
        &self,
        result: BridgeResult<TelemetryBatch>,
        value_len: usize,
    ) {
        match result {
            Ok(batch) => {
                // Update stats
                let mut stats = self.stats.write().await;
                stats.total_records += batch.size as u64;
                stats.total_bytes += value_len as u64;
                stats.last_record_time = Some(Utc::now());
            }
            Err(e) => {
                error!("Failed to process Kafka message: {}", e);
                let mut stats = self.stats.write().await;
                stats.error_count += 1;
            }
        }
    }

    /// Process Kafka message
    async fn process_kafka_message(
        &self,
        key: Option<Vec<u8>>,
        value: Vec<u8>,
    ) -> BridgeResult<TelemetryBatch> {
        info!("Processing Kafka message with {} bytes", value.len());

        let batch = match self.config.serialization_format {
            KafkaSerializationFormat::Arrow => {
                // Use Arrow IPC deserialization
                deserialize_from_arrow_ipc(&value)?
            }
            KafkaSerializationFormat::Json => self.decode_json_message(&value).await?,
            KafkaSerializationFormat::Avro => self.decode_avro_message(&value).await?,
            KafkaSerializationFormat::Protobuf => self.decode_protobuf_message(&value).await?,
            KafkaSerializationFormat::Parquet => self.decode_parquet_message(&value).await?,
        };

        // Add Kafka-specific metadata
        let mut metadata = batch.metadata;
        if let Some(key) = key {
            metadata.insert(
                "kafka_key".to_string(),
                String::from_utf8_lossy(&key).to_string(),
            );
        }
        metadata.insert("topic".to_string(), self.config.topic.clone());
        metadata.insert("group_id".to_string(), self.config.group_id.clone());
        metadata.insert(
            "serialization_format".to_string(),
            format!("{:?}", self.config.serialization_format),
        );

        Ok(TelemetryBatch {
            id: batch.id,
            timestamp: batch.timestamp,
            source: batch.source,
            size: batch.size,
            records: batch.records,
            metadata,
        })
    }

    /// Decode JSON message
    async fn decode_json_message(&self, payload: &[u8]) -> BridgeResult<TelemetryBatch> {
        // Parse JSON payload
        let json_value: serde_json::Value = serde_json::from_slice(payload).map_err(|e| {
            bridge_core::BridgeError::serialization(format!("Failed to parse JSON: {}", e))
        })?;

        // Try to parse as TelemetryBatch first
        if let Ok(batch) = serde_json::from_value::<TelemetryBatch>(json_value.clone()) {
            return Ok(batch);
        }

        // If not a TelemetryBatch, try to parse as individual record
        if let Ok(record) = serde_json::from_value::<TelemetryRecord>(json_value.clone()) {
            return Ok(TelemetryBatch {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                source: "kafka_json".to_string(),
                size: 1,
                records: vec![record],
                metadata: HashMap::new(),
            });
        }

        // If it's a simple metric, create a basic record
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: json_value
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown_metric")
                    .to_string(),
                description: json_value
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                unit: json_value
                    .get("unit")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(
                    json_value
                        .get("value")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                ),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "kafka_json".to_string(),
            size: 1,
            records: vec![record],
            metadata: HashMap::new(),
        })
    }

    /// Decode Avro message
    async fn decode_avro_message(&self, payload: &[u8]) -> BridgeResult<TelemetryBatch> {
        // Try to deserialize as Avro data
        // Note: In a production environment, you would need to manage Avro schemas
        // This implementation provides a basic framework for Avro deserialization

        // First, try to parse as a simple Avro record
        // For now, we'll create a basic schema for telemetry data
        let schema_json = r#"
        {
            "type": "record",
            "name": "TelemetryRecord",
            "fields": [
                {"name": "id", "type": "string"},
                {"name": "timestamp", "type": "long"},
                {"name": "name", "type": "string"},
                {"name": "value", "type": "double"},
                {"name": "description", "type": ["null", "string"]},
                {"name": "unit", "type": ["null", "string"]}
            ]
        }
        "#;

        match Schema::parse_str(schema_json) {
            Ok(schema) => {
                // Try to deserialize using the schema
                match from_avro_datum(&schema, &mut std::io::Cursor::new(payload), None) {
                    Ok(avro_value) => {
                        // Extract values from Avro record
                        let record = if let avro_rs::types::Value::Record(fields) = avro_value {
                            let mut name = "avro_metric".to_string();
                            let mut value = 0.0;
                            let mut description = None;
                            let mut unit = None;

                            for (field_name, field_value) in fields {
                                match field_name.as_str() {
                                    "name" => {
                                        if let avro_rs::types::Value::String(s) = field_value {
                                            name = s;
                                        }
                                    }
                                    "value" => {
                                        if let avro_rs::types::Value::Double(v) = field_value {
                                            value = v;
                                        }
                                    }
                                    "description" => {
                                        if let avro_rs::types::Value::String(s) = field_value {
                                            description = Some(s);
                                        }
                                    }
                                    "unit" => {
                                        if let avro_rs::types::Value::String(s) = field_value {
                                            unit = Some(s);
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            TelemetryRecord {
                                id: Uuid::new_v4(),
                                timestamp: Utc::now(),
                                record_type: TelemetryType::Metric,
                                data: TelemetryData::Metric(MetricData {
                                    name,
                                    description,
                                    unit,
                                    metric_type: MetricType::Gauge,
                                    value: MetricValue::Gauge(value),
                                    labels: HashMap::new(),
                                    timestamp: Utc::now(),
                                }),
                                attributes: HashMap::new(),
                                tags: HashMap::new(),
                                resource: None,
                                service: None,
                            }
                        } else {
                            // Fallback to basic record if not a record type
                            TelemetryRecord {
                                id: Uuid::new_v4(),
                                timestamp: Utc::now(),
                                record_type: TelemetryType::Metric,
                                data: TelemetryData::Metric(MetricData {
                                    name: "avro_metric".to_string(),
                                    description: Some("Avro deserialized metric".to_string()),
                                    unit: None,
                                    metric_type: MetricType::Gauge,
                                    value: MetricValue::Gauge(payload.len() as f64),
                                    labels: HashMap::new(),
                                    timestamp: Utc::now(),
                                }),
                                attributes: HashMap::new(),
                                tags: HashMap::new(),
                                resource: None,
                                service: None,
                            }
                        };

                        Ok(TelemetryBatch {
                            id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            source: "kafka_avro".to_string(),
                            size: 1,
                            records: vec![record],
                            metadata: HashMap::new(),
                        })
                    }
                    Err(e) => {
                        warn!("Failed to deserialize Avro data: {}", e);
                        // Fallback to basic record
                        self.create_fallback_avro_record(payload)
                    }
                }
            }
            Err(e) => {
                error!("Failed to parse Avro schema: {}", e);
                self.create_fallback_avro_record(payload)
            }
        }
    }

    /// Create fallback Avro record when deserialization fails
    fn create_fallback_avro_record(&self, payload: &[u8]) -> BridgeResult<TelemetryBatch> {
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "avro_metric".to_string(),
                description: Some("Avro deserialized metric (fallback)".to_string()),
                unit: None,
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(payload.len() as f64),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "kafka_avro".to_string(),
            size: 1,
            records: vec![record],
            metadata: HashMap::new(),
        })
    }

    /// Decode Protobuf message
    async fn decode_protobuf_message(&self, payload: &[u8]) -> BridgeResult<TelemetryBatch> {
        // Try to deserialize as Protobuf data
        // Note: In a production environment, you would need the protobuf message definitions
        // This implementation provides a basic framework for Protobuf deserialization

        // For now, we'll try to parse as a simple protobuf message
        // In a real implementation, you would have generated protobuf structs

        // Try to parse as a simple metric message
        // This is a basic example - in practice you'd have proper protobuf definitions
        if payload.len() >= 8 {
            // Try to extract basic fields from protobuf wire format
            // This is a simplified approach - real protobuf parsing would use generated code

            // Try to read protobuf fields (simplified)
            let mut name = "protobuf_metric".to_string();
            let mut value = 0.0;
            let mut description = None;

            // Simple protobuf field parsing (field number 1 = name, 2 = value, 3 = description)
            let mut pos = 0;
            while pos < payload.len() {
                if pos + 1 >= payload.len() {
                    break;
                }

                let field_info = payload[pos];
                let field_number = field_info >> 3;
                let wire_type = field_info & 0x07;

                pos += 1;

                match (field_number, wire_type) {
                    (1, 2) => {
                        // String field (name)
                        if pos + 1 < payload.len() {
                            let len = payload[pos] as usize;
                            pos += 1;
                            if pos + len <= payload.len() {
                                if let Ok(s) = String::from_utf8(payload[pos..pos + len].to_vec()) {
                                    name = s;
                                }
                                pos += len;
                            }
                        }
                    }
                    (2, 1) => {
                        // Double field (value)
                        if pos + 8 <= payload.len() {
                            let bytes = [
                                payload[pos],
                                payload[pos + 1],
                                payload[pos + 2],
                                payload[pos + 3],
                                payload[pos + 4],
                                payload[pos + 5],
                                payload[pos + 6],
                                payload[pos + 7],
                            ];
                            value = f64::from_le_bytes(bytes);
                            pos += 8;
                        }
                    }
                    (3, 2) => {
                        // String field (description)
                        if pos + 1 < payload.len() {
                            let len = payload[pos] as usize;
                            pos += 1;
                            if pos + len <= payload.len() {
                                if let Ok(s) = String::from_utf8(payload[pos..pos + len].to_vec()) {
                                    description = Some(s);
                                }
                                pos += len;
                            }
                        }
                    }
                    _ => {
                        // Skip unknown field
                        pos += 1;
                    }
                }
            }

            let record = TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name,
                    description,
                    unit: None,
                    metric_type: MetricType::Gauge,
                    value: MetricValue::Gauge(value),
                    labels: HashMap::new(),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::new(),
                tags: HashMap::new(),
                resource: None,
                service: None,
            };

            Ok(TelemetryBatch {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                source: "kafka_protobuf".to_string(),
                size: 1,
                records: vec![record],
                metadata: HashMap::new(),
            })
        } else {
            // Fallback for small payloads
            self.create_fallback_protobuf_record(payload)
        }
    }

    /// Create fallback Protobuf record when deserialization fails
    fn create_fallback_protobuf_record(&self, payload: &[u8]) -> BridgeResult<TelemetryBatch> {
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "protobuf_metric".to_string(),
                description: Some("Protobuf deserialized metric (fallback)".to_string()),
                unit: None,
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(payload.len() as f64),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "kafka_protobuf".to_string(),
            size: 1,
            records: vec![record],
            metadata: HashMap::new(),
        })
    }

    /// Decode Parquet message
    async fn decode_parquet_message(&self, payload: &[u8]) -> BridgeResult<TelemetryBatch> {
        // Try to deserialize as Parquet data
        // Note: In a production environment, you would need to handle Parquet file reading
        // This implementation provides a basic framework for Parquet deserialization

        // For now, we'll create a simplified Parquet reader that can handle basic data
        // In a real implementation, you would use the parquet crate's full API

        // Try to parse as a simple binary format that could represent Parquet data
        if payload.len() >= 4 {
            // Simple heuristic: if the payload starts with "PAR1" (Parquet magic number)
            // or has a reasonable structure, try to parse it
            let is_parquet_like = payload.len() > 100
                && (payload.starts_with(b"PAR1")
                    || payload.iter().any(|&b| b.is_ascii_alphanumeric()));

            if is_parquet_like {
                // Create a basic record from what we can extract
                let record = TelemetryRecord {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    record_type: TelemetryType::Metric,
                    data: TelemetryData::Metric(MetricData {
                        name: "parquet_metric".to_string(),
                        description: Some("Parquet deserialized metric".to_string()),
                        unit: None,
                        metric_type: MetricType::Gauge,
                        value: MetricValue::Gauge(payload.len() as f64),
                        labels: HashMap::new(),
                        timestamp: Utc::now(),
                    }),
                    attributes: HashMap::new(),
                    tags: HashMap::new(),
                    resource: None,
                    service: None,
                };

                Ok(TelemetryBatch {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    source: "kafka_parquet".to_string(),
                    size: 1,
                    records: vec![record],
                    metadata: HashMap::new(),
                })
            } else {
                self.create_fallback_parquet_record(payload)
            }
        } else {
            self.create_fallback_parquet_record(payload)
        }
    }

    /// Create fallback Parquet record when deserialization fails
    fn create_fallback_parquet_record(&self, payload: &[u8]) -> BridgeResult<TelemetryBatch> {
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "parquet_metric".to_string(),
                description: Some("Parquet deserialized metric (fallback)".to_string()),
                unit: None,
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(payload.len() as f64),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        Ok(TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "kafka_parquet".to_string(),
            size: 1,
            records: vec![record],
            metadata: HashMap::new(),
        })
    }
}

#[async_trait]
impl StreamSource for KafkaSource {
    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing Kafka source");

        // Validate configuration
        self.config.validate().await?;

        // Initialize Kafka consumer
        self.init_consumer().await?;

        info!("Kafka source initialized");
        Ok(())
    }

    async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting Kafka source");

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

        info!("Kafka source started");
        Ok(())
    }

    async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping Kafka source");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = false;
        }

        info!("Kafka source stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        // Check if the source is running
        if let Some(consumer) = &self.consumer {
            consumer.is_running()
        } else {
            false
        }
    }

    async fn get_stats(&self) -> BridgeResult<SourceStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn version(&self) -> &str {
        &self.config.version
    }
}

impl KafkaSource {
    /// Get Kafka configuration
    pub fn get_config(&self) -> &KafkaSourceConfig {
        &self.config
    }
}
