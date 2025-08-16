//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Kafka sink implementation
//!
//! This module provides a Kafka sink for streaming data to Kafka topics.

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
// Mock Kafka implementation for now - rdkafka has build issues
// In production, you would use rdkafka::{
//     producer::{FutureProducer, FutureRecord},
//     ClientConfig,
//     error::KafkaError,
//     message::OwnedHeaders,
// };

use super::{SinkConfig, SinkStats, StreamSink};
use crate::arrow_utils::serialize_to_arrow_ipc;

/// Kafka sink configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaSinkConfig {
    /// Sink name
    pub name: String,

    /// Sink version
    pub version: String,

    /// Kafka bootstrap servers
    pub bootstrap_servers: Vec<String>,

    /// Topic to produce to
    pub topic: String,

    /// Producer client ID
    pub client_id: String,

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

impl KafkaSinkConfig {
    /// Create new Kafka sink configuration
    pub fn new(bootstrap_servers: Vec<String>, topic: String) -> Self {
        Self {
            name: "kafka".to_string(),
            version: "1.0.0".to_string(),
            bootstrap_servers,
            topic,
            client_id: "streaming-processor-sink".to_string(),
            serialization_format: KafkaSerializationFormat::Json,
            security: None,
            properties: HashMap::new(),
        }
    }
}

#[async_trait]
impl SinkConfig for KafkaSinkConfig {
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

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Kafka sink implementation
pub struct KafkaSink {
    config: KafkaSinkConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<SinkStats>>,
    producer: Option<KafkaProducer>,
}

/// Kafka producer wrapper
struct KafkaProducer {
    topic: String,
    // In production, this would be: producer: FutureProducer,
}

impl KafkaProducer {
    /// Create a new Kafka producer
    async fn new(config: &KafkaSinkConfig) -> BridgeResult<Self> {
        // Mock implementation - in production this would use rdkafka
        info!("Creating mock Kafka producer for topic: {}", config.topic);

        // Validate configuration
        if config.bootstrap_servers.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Bootstrap servers cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            topic: config.topic.clone(),
        })
    }

    /// Send message to Kafka
    async fn send_message(&self, key: Option<Vec<u8>>, value: Vec<u8>) -> BridgeResult<()> {
        // Mock implementation - in production this would use rdkafka
        let key_str = key
            .as_ref()
            .and_then(|k| String::from_utf8(k.clone()).ok())
            .unwrap_or_else(|| "no_key".to_string());

        info!(
            "Mock: Sending message to Kafka topic {} with key {} ({} bytes)",
            self.topic,
            key_str,
            value.len()
        );

        // Simulate network delay
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        info!(
            "Mock: Message sent to Kafka topic {} partition 0 offset 123",
            self.topic
        );

        Ok(())
    }
}

impl KafkaSink {
    /// Create new Kafka sink
    pub async fn new(config: &dyn SinkConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<KafkaSinkConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration(
                    "Invalid Kafka sink configuration".to_string(),
                )
            })?
            .clone();

        config.validate().await?;

        let stats = SinkStats {
            sink: config.name.clone(),
            total_batches: 0,
            total_records: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            total_bytes: 0,
            bytes_per_minute: 0,
            error_count: 0,
            last_send_time: None,
            is_connected: false,
            latency_ms: 0,
        };

        Ok(Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
            producer: None,
        })
    }

    /// Initialize Kafka producer
    async fn init_producer(&mut self) -> BridgeResult<()> {
        info!(
            "Initializing Kafka producer for topic: {}",
            self.config.topic
        );

        let producer = KafkaProducer::new(&self.config).await?;
        self.producer = Some(producer);

        info!("Kafka producer initialized");
        Ok(())
    }

    /// Send batch to Kafka
    async fn send_batch(&self, batch: TelemetryBatch) -> BridgeResult<()> {
        let start_time = std::time::Instant::now();

        info!("Sending batch to Kafka topic: {}", self.config.topic);

        // Serialize batch based on format
        let serialized_data = match self.config.serialization_format {
            KafkaSerializationFormat::Arrow => {
                // Use Arrow IPC serialization
                serialize_to_arrow_ipc(&batch)?
            }
            KafkaSerializationFormat::Json => self.serialize_to_json(&batch).await?,
            KafkaSerializationFormat::Avro => self.serialize_to_avro(&batch).await?,
            KafkaSerializationFormat::Protobuf => self.serialize_to_protobuf(&batch).await?,
            KafkaSerializationFormat::Parquet => self.serialize_to_parquet(&batch).await?,
        };

        // Send to Kafka
        let data_len = serialized_data.len();
        if let Some(producer) = &self.producer {
            let key = Some(batch.id.to_string().into_bytes());
            producer.send_message(key, serialized_data).await?;
        } else {
            return Err(bridge_core::BridgeError::stream(
                "Kafka producer not initialized".to_string(),
            ));
        }

        let send_time = start_time.elapsed().as_millis() as u64;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_batches += 1;
            stats.total_records += batch.size as u64;
            stats.total_bytes += data_len as u64;
            stats.last_send_time = Some(Utc::now());
            stats.latency_ms = send_time;
            stats.is_connected = true;
        }

        info!(
            "Batch sent to Kafka in {}ms ({} bytes)",
            send_time, data_len
        );
        Ok(())
    }

    /// Serialize to JSON
    async fn serialize_to_json(&self, batch: &TelemetryBatch) -> BridgeResult<Vec<u8>> {
        let json_data = serde_json::to_vec(batch).map_err(|e| {
            bridge_core::BridgeError::serialization(format!("Failed to serialize to JSON: {}", e))
        })?;
        Ok(json_data)
    }

    /// Serialize to Avro
    async fn serialize_to_avro(&self, batch: &TelemetryBatch) -> BridgeResult<Vec<u8>> {
        // For now, we'll serialize as JSON and wrap it in a simple Avro structure
        // In a production environment, you would use proper Avro schemas
        let json_data = self.serialize_to_json(batch).await?;

        // Create a simple Avro record structure
        let _avro_schema = r#"
        {
            "type": "record",
            "name": "TelemetryBatch",
            "fields": [
                {"name": "data", "type": "bytes"},
                {"name": "timestamp", "type": "long"},
                {"name": "format", "type": "string"}
            ]
        }
        "#;

        // For simplicity, we'll just return the JSON data as bytes
        // In a real implementation, you would use avro-rs to properly serialize
        Ok(json_data)
    }

    /// Serialize to Protobuf
    async fn serialize_to_protobuf(&self, batch: &TelemetryBatch) -> BridgeResult<Vec<u8>> {
        // For now, we'll serialize as JSON and encode it as a simple protobuf message
        // In a production environment, you would use proper protobuf definitions
        let json_data = self.serialize_to_json(batch).await?;

        // Create a simple protobuf-like structure
        // This is a simplified approach - in practice you'd use prost-generated structs
        let mut protobuf_data = Vec::new();

        // Add field 1 (string): JSON data as bytes
        protobuf_data.push(0x0a); // Field 1, wire type 2 (string)
        protobuf_data.extend_from_slice(&(json_data.len() as u32).to_le_bytes());
        protobuf_data.extend_from_slice(&json_data);

        // Add field 2 (int64): timestamp
        let timestamp = batch.timestamp.timestamp();
        protobuf_data.push(0x10); // Field 2, wire type 0 (varint)
        protobuf_data.extend_from_slice(&timestamp.to_le_bytes());

        Ok(protobuf_data)
    }

    /// Serialize to Parquet
    async fn serialize_to_parquet(&self, batch: &TelemetryBatch) -> BridgeResult<Vec<u8>> {
        // For now, we'll use Arrow IPC as a proxy for Parquet
        // In a production environment, you would use the parquet crate directly
        let arrow_data = serialize_to_arrow_ipc(batch)?;

        // Convert Arrow IPC to Parquet format
        // This is a simplified approach - in practice you'd use arrow::parquet
        Ok(arrow_data)
    }
}

#[async_trait]
impl StreamSink for KafkaSink {
    async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing Kafka sink");

        // Validate configuration
        self.config.validate().await?;

        // Initialize Kafka producer
        self.init_producer().await?;

        info!("Kafka sink initialized");
        Ok(())
    }

    async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting Kafka sink");

        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = true;
        }

        info!("Kafka sink started");
        Ok(())
    }

    async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping Kafka sink");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_connected = false;
        }

        info!("Kafka sink stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        // This is a simplified check - in practice we'd need to handle the async nature
        false
    }

    async fn send(&self, batch: TelemetryBatch) -> BridgeResult<()> {
        if !self.is_running().await {
            return Err(bridge_core::BridgeError::stream(
                "Kafka sink is not running".to_string(),
            ));
        }

        self.send_batch(batch).await
    }

    async fn get_stats(&self) -> BridgeResult<SinkStats> {
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

impl KafkaSink {
    /// Get Kafka configuration
    pub fn get_config(&self) -> &KafkaSinkConfig {
        &self.config
    }

    /// Check if sink is running
    pub async fn is_running(&self) -> bool {
        let is_running = self.is_running.read().await;
        *is_running
    }
}
