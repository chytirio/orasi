//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Kafka sink implementation
//!
//! This module provides a Kafka sink for streaming data to Kafka topics.

use async_trait::async_trait;
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::Utc;
// use rdkafka::{
//     producer::{FutureProducer, FutureRecord},
//     ClientConfig,
//     error::KafkaError,
//     message::OwnedHeaders,
// };
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

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

    /// Producer configuration
    pub producer_config: ProducerConfig,
}

/// Producer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProducerConfig {
    /// Acks configuration
    pub acks: String,

    /// Retry configuration
    pub retries: i32,

    /// Batch size in bytes
    pub batch_size: usize,

    /// Linger time in milliseconds
    pub linger_ms: u64,

    /// Buffer memory in bytes
    pub buffer_memory: usize,

    /// Compression type
    pub compression_type: String,

    /// Max request size in bytes
    pub max_request_size: usize,

    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
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
            producer_config: ProducerConfig::default(),
        }
    }
}

impl Default for ProducerConfig {
    fn default() -> Self {
        Self {
            acks: "all".to_string(),
            retries: 3,
            batch_size: 16384,
            linger_ms: 5,
            buffer_memory: 33554432, // 32MB
            compression_type: "snappy".to_string(),
            max_request_size: 1048576, // 1MB
            request_timeout_ms: 30000,
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

        if self.client_id.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Kafka client ID cannot be empty".to_string(),
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

/// Kafka producer wrapper (mock implementation)
struct KafkaProducer {
    topic: String,
    // producer: FutureProducer, // Uncomment when rdkafka is available
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

        // Mock implementation - in production this would use rdkafka
        info!(
            "Creating mock Kafka producer for topic: {}",
            self.config.topic
        );

        // Validate configuration
        if self.config.bootstrap_servers.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Bootstrap servers cannot be empty".to_string(),
            ));
        }

        self.producer = Some(KafkaProducer {
            topic: self.config.topic.clone(),
        });

        info!("Kafka producer initialized successfully");
        Ok(())
    }

    /// Serialize telemetry batch based on format
    async fn serialize_batch(&self, batch: &TelemetryBatch) -> BridgeResult<Vec<u8>> {
        match self.config.serialization_format {
            KafkaSerializationFormat::Json => serde_json::to_vec(batch).map_err(|e| {
                bridge_core::BridgeError::serialization(format!(
                    "Failed to serialize to JSON: {}",
                    e
                ))
            }),
            KafkaSerializationFormat::Arrow => serialize_to_arrow_ipc(batch).map_err(|e| {
                bridge_core::BridgeError::serialization(format!(
                    "Failed to serialize to Arrow: {}",
                    e
                ))
            }),
            KafkaSerializationFormat::Avro => {
                // For now, fallback to JSON for Avro
                // In production, you would use proper Avro serialization
                warn!("Avro serialization not fully implemented, falling back to JSON");
                serde_json::to_vec(batch).map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to serialize to JSON: {}",
                        e
                    ))
                })
            }
            KafkaSerializationFormat::Protobuf => {
                // For now, fallback to JSON for Protobuf
                // In production, you would use proper Protobuf serialization
                warn!("Protobuf serialization not fully implemented, falling back to JSON");
                serde_json::to_vec(batch).map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to serialize to JSON: {}",
                        e
                    ))
                })
            }
            KafkaSerializationFormat::Parquet => {
                // For now, fallback to JSON for Parquet
                // In production, you would use proper Parquet serialization
                warn!("Parquet serialization not fully implemented, falling back to JSON");
                serde_json::to_vec(batch).map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to serialize to JSON: {}",
                        e
                    ))
                })
            }
        }
    }

    /// Generate message key for the batch
    fn generate_message_key(&self, batch: &TelemetryBatch) -> Option<String> {
        // Generate a key based on batch properties
        // In production, you might want to use a specific field or generate a hash
        Some(format!("{}-{}", batch.source, batch.timestamp.timestamp()))
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
        // Check if the sink is running
        if let Some(_producer) = &self.producer {
            // In a real implementation, you would check the producer's health
            true
        } else {
            false
        }
    }

    async fn send(&self, batch: TelemetryBatch) -> BridgeResult<()> {
        let start_time = std::time::Instant::now();

        info!("Sending batch to Kafka topic: {}", self.config.topic);

        // Serialize the batch
        let payload = self.serialize_batch(&batch).await?;
        let key = self.generate_message_key(&batch);

        // Mock send operation
        let key_str = key.unwrap_or_else(|| "no_key".to_string());

        info!(
            "Mock: Sending message to Kafka topic {} with key {} ({} bytes)",
            self.config.topic,
            key_str,
            payload.len()
        );

        // Simulate network delay
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let latency = start_time.elapsed().as_millis() as u64;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_batches += 1;
            stats.total_records += batch.size as u64;
            stats.total_bytes += payload.len() as u64;
            stats.last_send_time = Some(Utc::now());
            stats.latency_ms = latency;
        }

        info!(
            "Mock: Message sent to Kafka topic {} partition 0 offset 123 latency: {}ms",
            self.config.topic, latency
        );

        Ok(())
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
}
