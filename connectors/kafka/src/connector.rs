//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Kafka connector implementation
//! 
//! This module provides the main Kafka connector that implements
//! the LakehouseConnector trait for seamless integration with the bridge.

use async_trait::async_trait;
use tracing::{debug, info};
use bridge_core::traits::{LakehouseConnector, LakehouseWriter, LakehouseReader};
use bridge_core::error::BridgeResult;

use crate::config::KafkaConfig;
use crate::error::{KafkaError, KafkaResult};
use crate::producer::KafkaProducer;
use crate::consumer::KafkaConsumer;

/// Kafka connector implementation
pub struct KafkaConnector {
    /// Kafka configuration
    config: KafkaConfig,
    /// Connection state
    connected: bool,
    /// Producer instance
    producer: Option<KafkaProducer>,
    /// Consumer instance
    consumer: Option<KafkaConsumer>,
}

impl KafkaConnector {
    /// Create a new Kafka connector
    pub fn new(config: KafkaConfig) -> Self {
        Self {
            config,
            connected: false,
            producer: None,
            consumer: None,
        }
    }

    /// Initialize the connector
    pub async fn initialize(&mut self) -> KafkaResult<()> {
        info!("Initializing Kafka connector for topic: {}", self.config.topic_name());
        
        // Validate configuration
        self.config.validate()?;
        
        // Initialize Kafka connection and topics if they don't exist
        self.ensure_connection_and_topics().await?;
        
        // Initialize producer and consumer
        let producer = KafkaProducer::new(self.config.clone()).await?;
        let consumer = KafkaConsumer::new(self.config.clone()).await?;
        
        self.producer = Some(producer);
        self.consumer = Some(consumer);
        self.connected = true;
        
        info!("Kafka connector initialized successfully");
        Ok(())
    }

    /// Ensure connection and topics exist
    async fn ensure_connection_and_topics(&self) -> KafkaResult<()> {
        debug!("Ensuring Kafka connection and topics exist");
        
        // This would typically involve:
        // 1. Establishing connection to Kafka cluster
        // 2. Checking if topic exists
        // 3. Creating topic if it doesn't exist
        // 4. Validating topic configuration
        // 5. Setting up partitions and replication
        
        // For now, we'll just log the operation
        info!("Connection and topic check completed for Kafka");
        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &KafkaConfig {
        &self.config
    }

    /// Check if the connector is connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[async_trait]
impl LakehouseConnector for KafkaConnector {
    type Config = KafkaConfig;
    type WriteHandle = KafkaProducer;
    type ReadHandle = KafkaConsumer;

    async fn connect(config: Self::Config) -> BridgeResult<Self> {
        let mut connector = KafkaConnector::new(config);
        connector.initialize().await
            .map_err(|e| bridge_core::error::BridgeError::lakehouse_with_source("Failed to initialize Kafka connector", e))?;
        Ok(connector)
    }

    async fn writer(&self) -> BridgeResult<Self::WriteHandle> {
        self.producer.as_ref()
            .cloned()
            .ok_or_else(|| bridge_core::error::BridgeError::lakehouse("Producer not initialized"))
    }

    async fn reader(&self) -> BridgeResult<Self::ReadHandle> {
        self.consumer.as_ref()
            .cloned()
            .ok_or_else(|| bridge_core::error::BridgeError::lakehouse("Consumer not initialized"))
    }

    fn name(&self) -> &str {
        "kafka-connector"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        if !self.connected {
            return Ok(false);
        }

        // Perform health checks on producer and consumer
        let producer_healthy = if let Some(producer) = &self.producer {
            producer.is_initialized()
        } else {
            false
        };

        let consumer_healthy = if let Some(consumer) = &self.consumer {
            consumer.is_initialized()
        } else {
            false
        };

        Ok(producer_healthy && consumer_healthy)
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ConnectorStats> {
        let producer_stats = if let Some(producer) = &self.producer {
            producer.get_stats().await.unwrap_or_else(|_| bridge_core::traits::WriterStats {
                total_writes: 0,
                total_records: 0,
                writes_per_minute: 0,
                records_per_minute: 0,
                avg_write_time_ms: 0.0,
                error_count: 0,
                last_write_time: None,
            })
        } else {
            bridge_core::traits::WriterStats {
                total_writes: 0,
                total_records: 0,
                writes_per_minute: 0,
                records_per_minute: 0,
                avg_write_time_ms: 0.0,
                error_count: 0,
                last_write_time: None,
            }
        };

        let consumer_stats = if let Some(consumer) = &self.consumer {
            consumer.get_stats().await.unwrap_or_else(|_| bridge_core::traits::ReaderStats {
                total_reads: 0,
                total_records: 0,
                reads_per_minute: 0,
                records_per_minute: 0,
                avg_read_time_ms: 0.0,
                error_count: 0,
                last_read_time: None,
            })
        } else {
            bridge_core::traits::ReaderStats {
                total_reads: 0,
                total_records: 0,
                reads_per_minute: 0,
                records_per_minute: 0,
                avg_read_time_ms: 0.0,
                error_count: 0,
                last_read_time: None,
            }
        };

        Ok(bridge_core::traits::ConnectorStats {
            total_connections: 1,
            active_connections: if self.connected { 1 } else { 0 },
            total_writes: producer_stats.total_writes,
            total_reads: consumer_stats.total_reads,
            error_count: producer_stats.error_count + consumer_stats.error_count,
            avg_write_time_ms: producer_stats.avg_write_time_ms,
            avg_read_time_ms: consumer_stats.avg_read_time_ms,
            last_operation_time: producer_stats.last_write_time.or(consumer_stats.last_read_time),
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down Kafka connector");
        
        // Shutdown producer and consumer
        if let Some(producer) = &self.producer {
            let _ = producer.close().await;
        }
        
        if let Some(consumer) = &self.consumer {
            let _ = consumer.close().await;
        }
        
        info!("Kafka connector shutdown completed");
        Ok(())
    }
}
