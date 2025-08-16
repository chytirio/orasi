//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Kafka configuration management
//! 
//! This module provides configuration structures and management
//! for Kafka connector settings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use crate::error::{KafkaError, KafkaResult};

/// Kafka authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaAuth {
    /// Authentication mechanism (PLAIN, SCRAM-SHA-256, etc.)
    pub mechanism: String,
    /// Username for authentication
    pub username: Option<String>,
    /// Password for authentication
    pub password: Option<String>,
    /// SSL/TLS configuration
    pub ssl: Option<KafkaSSL>,
}

/// Kafka SSL/TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaSSL {
    /// SSL certificate path
    pub certificate_path: Option<String>,
    /// SSL key path
    pub key_path: Option<String>,
    /// SSL CA path
    pub ca_path: Option<String>,
    /// SSL password
    pub password: Option<String>,
    /// Enable SSL verification
    pub verify: bool,
}

/// Kafka cluster configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaCluster {
    /// Bootstrap servers
    pub bootstrap_servers: String,
    /// Client ID
    pub client_id: String,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    /// Socket timeout in milliseconds
    pub socket_timeout_ms: u64,
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
    /// Metadata max age in milliseconds
    pub metadata_max_age_ms: u64,
    /// Reconnect backoff in milliseconds
    pub reconnect_backoff_ms: u64,
    /// Retry backoff in milliseconds
    pub retry_backoff_ms: u64,
}

/// Kafka producer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaProducerConfig {
    /// Batch size for producer
    pub batch_size: usize,
    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,
    /// Compression type (none, gzip, snappy, lz4, zstd)
    pub compression_type: String,
    /// Acks configuration (0, 1, all)
    pub acks: String,
    /// Retry count
    pub retries: u32,
    /// Max in flight requests
    pub max_in_flight_requests: u32,
    /// Enable idempotence
    pub enable_idempotence: bool,
    /// Max request size
    pub max_request_size: usize,
    /// Buffer memory
    pub buffer_memory: usize,
    /// Linger time in milliseconds
    pub linger_ms: u64,
    /// Batch size in bytes
    pub batch_size_bytes: usize,
}

/// Kafka consumer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConsumerConfig {
    /// Consumer group ID
    pub group_id: String,
    /// Auto offset reset (earliest, latest)
    pub auto_offset_reset: String,
    /// Enable auto commit
    pub enable_auto_commit: bool,
    /// Auto commit interval in milliseconds
    pub auto_commit_interval_ms: u64,
    /// Session timeout in milliseconds
    pub session_timeout_ms: u64,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
    /// Max poll records
    pub max_poll_records: usize,
    /// Max poll interval in milliseconds
    pub max_poll_interval_ms: u64,
    /// Fetch min bytes
    pub fetch_min_bytes: usize,
    /// Fetch max wait time in milliseconds
    pub fetch_max_wait_ms: u64,
    /// Enable partition EOF
    pub enable_partition_eof: bool,
}

/// Kafka topic configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaTopic {
    /// Topic name
    pub name: String,
    /// Number of partitions
    pub partitions: i32,
    /// Replication factor
    pub replication_factor: i16,
    /// Topic configuration
    pub config: HashMap<String, String>,
}

/// Main Kafka configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Cluster configuration
    pub cluster: KafkaCluster,
    /// Authentication configuration
    pub auth: Option<KafkaAuth>,
    /// Producer configuration
    pub producer: KafkaProducerConfig,
    /// Consumer configuration
    pub consumer: KafkaConsumerConfig,
    /// Topic configuration
    pub topic: KafkaTopic,
    /// Additional settings
    pub settings: HashMap<String, String>,
}

impl KafkaConfig {
    /// Create a new Kafka configuration
    pub fn new(
        bootstrap_servers: String,
        topic_name: String,
        group_id: String,
    ) -> Self {
        Self {
            cluster: KafkaCluster {
                bootstrap_servers,
                client_id: "otel-bridge-client".to_string(),
                request_timeout_ms: 30000,
                socket_timeout_ms: 30000,
                connection_timeout_ms: 60000,
                metadata_max_age_ms: 300000,
                reconnect_backoff_ms: 50,
                retry_backoff_ms: 100,
            },
            auth: None,
            producer: KafkaProducerConfig {
                batch_size: crate::DEFAULT_PRODUCER_BATCH_SIZE,
                flush_interval_ms: crate::DEFAULT_PRODUCER_FLUSH_INTERVAL_MS,
                compression_type: "snappy".to_string(),
                acks: "all".to_string(),
                retries: 3,
                max_in_flight_requests: 5,
                enable_idempotence: true,
                max_request_size: 1048576, // 1MB
                buffer_memory: 33554432,   // 32MB
                linger_ms: 5,
                batch_size_bytes: 16384,   // 16KB
            },
            consumer: KafkaConsumerConfig {
                group_id,
                auto_offset_reset: crate::DEFAULT_CONSUMER_AUTO_OFFSET_RESET.to_string(),
                enable_auto_commit: crate::DEFAULT_CONSUMER_ENABLE_AUTO_COMMIT,
                auto_commit_interval_ms: crate::DEFAULT_CONSUMER_AUTO_COMMIT_INTERVAL_MS,
                session_timeout_ms: crate::DEFAULT_SESSION_TIMEOUT_MS,
                heartbeat_interval_ms: crate::DEFAULT_HEARTBEAT_INTERVAL_MS,
                max_poll_records: crate::DEFAULT_MAX_POLL_RECORDS,
                max_poll_interval_ms: crate::DEFAULT_MAX_POLL_INTERVAL_MS,
                fetch_min_bytes: 1,
                fetch_max_wait_ms: 500,
                enable_partition_eof: false,
            },
            topic: KafkaTopic {
                name: topic_name,
                partitions: 3,
                replication_factor: 1,
                config: HashMap::new(),
            },
            settings: HashMap::new(),
        }
    }

    /// Get the topic name
    pub fn topic_name(&self) -> &str {
        &self.topic.name
    }

    /// Get the bootstrap servers
    pub fn bootstrap_servers(&self) -> &str {
        &self.cluster.bootstrap_servers
    }

    /// Get the client ID
    pub fn client_id(&self) -> &str {
        &self.cluster.client_id
    }

    /// Get the consumer group ID
    pub fn group_id(&self) -> &str {
        &self.consumer.group_id
    }

    /// Get the batch size
    pub fn batch_size(&self) -> usize {
        self.producer.batch_size
    }

    /// Get the flush interval
    pub fn flush_interval_ms(&self) -> u64 {
        self.producer.flush_interval_ms
    }

    /// Get the compression type
    pub fn compression_type(&self) -> &str {
        &self.producer.compression_type
    }

    /// Get the number of partitions
    pub fn partitions(&self) -> i32 {
        self.topic.partitions
    }

    /// Get the replication factor
    pub fn replication_factor(&self) -> i16 {
        self.topic.replication_factor
    }

    /// Validate the configuration
    pub fn validate(&self) -> KafkaResult<()> {
        // Validate cluster settings
        if self.cluster.bootstrap_servers.is_empty() {
            return Err(KafkaError::configuration("Bootstrap servers cannot be empty"));
        }

        if self.cluster.client_id.is_empty() {
            return Err(KafkaError::configuration("Client ID cannot be empty"));
        }

        // Validate producer settings
        if self.producer.batch_size == 0 {
            return Err(KafkaError::configuration("Batch size cannot be zero"));
        }

        if self.producer.flush_interval_ms == 0 {
            return Err(KafkaError::configuration("Flush interval cannot be zero"));
        }

        if self.producer.compression_type.is_empty() {
            return Err(KafkaError::configuration("Compression type cannot be empty"));
        }

        // Validate consumer settings
        if self.consumer.group_id.is_empty() {
            return Err(KafkaError::configuration("Consumer group ID cannot be empty"));
        }

        if self.consumer.auto_offset_reset.is_empty() {
            return Err(KafkaError::configuration("Auto offset reset cannot be empty"));
        }

        // Validate topic settings
        if self.topic.name.is_empty() {
            return Err(KafkaError::configuration("Topic name cannot be empty"));
        }

        if self.topic.partitions <= 0 {
            return Err(KafkaError::configuration("Number of partitions must be positive"));
        }

        if self.topic.replication_factor <= 0 {
            return Err(KafkaError::configuration("Replication factor must be positive"));
        }

        Ok(())
    }

    /// Get producer configuration as HashMap
    pub fn producer_config(&self) -> HashMap<String, String> {
        let mut config = HashMap::new();
        
        config.insert("bootstrap.servers".to_string(), self.cluster.bootstrap_servers.clone());
        config.insert("client.id".to_string(), self.cluster.client_id.clone());
        config.insert("batch.size".to_string(), self.producer.batch_size.to_string());
        config.insert("linger.ms".to_string(), self.producer.linger_ms.to_string());
        config.insert("compression.type".to_string(), self.producer.compression_type.clone());
        config.insert("acks".to_string(), self.producer.acks.clone());
        config.insert("retries".to_string(), self.producer.retries.to_string());
        config.insert("max.in.flight.requests.per.connection".to_string(), self.producer.max_in_flight_requests.to_string());
        config.insert("enable.idempotence".to_string(), self.producer.enable_idempotence.to_string());
        config.insert("max.request.size".to_string(), self.producer.max_request_size.to_string());
        config.insert("buffer.memory".to_string(), self.producer.buffer_memory.to_string());
        config.insert("batch.size.bytes".to_string(), self.producer.batch_size_bytes.to_string());
        
        // Add authentication if configured
        if let Some(auth) = &self.auth {
            config.insert("security.protocol".to_string(), "SASL_SSL".to_string());
            config.insert("sasl.mechanism".to_string(), auth.mechanism.clone());
            if let Some(username) = &auth.username {
                config.insert("sasl.username".to_string(), username.clone());
            }
            if let Some(password) = &auth.password {
                config.insert("sasl.password".to_string(), password.clone());
            }
        }
        
        config
    }

    /// Get consumer configuration as HashMap
    pub fn consumer_config(&self) -> HashMap<String, String> {
        let mut config = HashMap::new();
        
        config.insert("bootstrap.servers".to_string(), self.cluster.bootstrap_servers.clone());
        config.insert("client.id".to_string(), self.cluster.client_id.clone());
        config.insert("group.id".to_string(), self.consumer.group_id.clone());
        config.insert("auto.offset.reset".to_string(), self.consumer.auto_offset_reset.clone());
        config.insert("enable.auto.commit".to_string(), self.consumer.enable_auto_commit.to_string());
        config.insert("auto.commit.interval.ms".to_string(), self.consumer.auto_commit_interval_ms.to_string());
        config.insert("session.timeout.ms".to_string(), self.consumer.session_timeout_ms.to_string());
        config.insert("heartbeat.interval.ms".to_string(), self.consumer.heartbeat_interval_ms.to_string());
        config.insert("max.poll.records".to_string(), self.consumer.max_poll_records.to_string());
        config.insert("max.poll.interval.ms".to_string(), self.consumer.max_poll_interval_ms.to_string());
        config.insert("fetch.min.bytes".to_string(), self.consumer.fetch_min_bytes.to_string());
        config.insert("fetch.max.wait.ms".to_string(), self.consumer.fetch_max_wait_ms.to_string());
        config.insert("enable.partition.eof".to_string(), self.consumer.enable_partition_eof.to_string());
        
        // Add authentication if configured
        if let Some(auth) = &self.auth {
            config.insert("security.protocol".to_string(), "SASL_SSL".to_string());
            config.insert("sasl.mechanism".to_string(), auth.mechanism.clone());
            if let Some(username) = &auth.username {
                config.insert("sasl.username".to_string(), username.clone());
            }
            if let Some(password) = &auth.password {
                config.insert("sasl.password".to_string(), password.clone());
            }
        }
        
        config
    }
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self::new(
            crate::DEFAULT_BOOTSTRAP_SERVERS.to_string(),
            crate::DEFAULT_TOPIC.to_string(),
            crate::DEFAULT_CONSUMER_GROUP_ID.to_string(),
        )
    }
}
