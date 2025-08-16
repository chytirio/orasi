//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Kafka connector for OpenTelemetry Data Lake Bridge
//!
//! This module provides integration with Apache Kafka for streaming
//! telemetry data in a distributed messaging system.

pub mod config;
pub mod connector;
pub mod error;
pub mod producer;
pub mod consumer;
pub mod cluster;
pub mod topic;
pub mod metrics;
pub mod exporter;

// Re-export main types
pub use bridge_core::traits::{LakehouseConnector, LakehouseReader, LakehouseWriter, LakehouseExporter};
pub use bridge_core::types::{ExportResult, ProcessedBatch, TelemetryBatch};
pub use config::KafkaConfig;
pub use connector::KafkaConnector;
pub use error::{KafkaError, KafkaResult};
pub use producer::KafkaProducer;
pub use consumer::KafkaConsumer;
pub use cluster::KafkaCluster;
pub use topic::KafkaTopic;
pub use metrics::KafkaMetrics;
pub use exporter::RealKafkaExporter;

/// Kafka connector version
pub const KAFKA_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Kafka connector name
pub const KAFKA_NAME: &str = "kafka-connector";
/// Default Kafka bootstrap servers
pub const DEFAULT_BOOTSTRAP_SERVERS: &str = "localhost:9092";
/// Default Kafka topic
pub const DEFAULT_TOPIC: &str = "telemetry";
/// Default Kafka producer batch size
pub const DEFAULT_PRODUCER_BATCH_SIZE: usize = 1000;
/// Default Kafka producer flush interval in milliseconds
pub const DEFAULT_PRODUCER_FLUSH_INTERVAL_MS: u64 = 1000;
/// Default Kafka consumer group ID
pub const DEFAULT_CONSUMER_GROUP_ID: &str = "otel-bridge-consumer";
/// Default Kafka consumer auto offset reset
pub const DEFAULT_CONSUMER_AUTO_OFFSET_RESET: &str = "latest";
/// Default Kafka consumer enable auto commit
pub const DEFAULT_CONSUMER_ENABLE_AUTO_COMMIT: bool = true;
/// Default Kafka consumer auto commit interval
pub const DEFAULT_CONSUMER_AUTO_COMMIT_INTERVAL_MS: u64 = 5000;
/// Default Kafka session timeout
pub const DEFAULT_SESSION_TIMEOUT_MS: u64 = 30000;
/// Default Kafka heartbeat interval
pub const DEFAULT_HEARTBEAT_INTERVAL_MS: u64 = 3000;
/// Default Kafka max poll records
pub const DEFAULT_MAX_POLL_RECORDS: usize = 500;
/// Default Kafka max poll interval
pub const DEFAULT_MAX_POLL_INTERVAL_MS: u64 = 300000;

/// Initialize Kafka connector with default configuration
pub async fn init_kafka_connector() -> anyhow::Result<()> {
    tracing::info!("Initializing Apache Kafka connector v{}", KAFKA_VERSION);
    tracing::info!("Apache Kafka connector initialization completed");
    Ok(())
}

/// Shutdown Kafka connector gracefully
pub async fn shutdown_kafka_connector() -> anyhow::Result<()> {
    tracing::info!("Shutting down Apache Kafka connector");
    tracing::info!("Apache Kafka connector shutdown completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kafka_connector_initialization() {
        let result = init_kafka_connector().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_kafka_connector_shutdown() {
        let result = shutdown_kafka_connector().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_kafka_connector_basic_functionality() {
        // Create a simple Kafka configuration
        let config = KafkaConfig::new(
            "localhost:9092".to_string(),
            "test-topic".to_string(),
            "test-consumer".to_string(),
        );

        // Test that we can create a connector
        let connector = KafkaConnector::new(config);
        assert_eq!(connector.name(), "kafka-connector");
        assert_eq!(connector.version(), KAFKA_VERSION);
        assert!(!connector.is_connected());

        // Test health check (should be false when not connected)
        let health = connector.health_check().await.unwrap();
        assert!(!health);
    }

    #[tokio::test]
    async fn test_kafka_producer_basic_functionality() {
        let config = KafkaConfig::new(
            "localhost:9092".to_string(),
            "test-topic".to_string(),
            "test-consumer".to_string(),
        );

        // Test producer creation
        let producer = KafkaProducer::new(config).await;
        assert!(producer.is_ok());
        let producer = producer.unwrap();
        assert!(producer.is_initialized());

        // Test producer stats
        let stats = producer.get_stats().await;
        assert!(stats.is_ok());
        let stats = stats.unwrap();
        assert_eq!(stats.total_writes, 0);
        assert_eq!(stats.total_records, 0);
    }

    #[tokio::test]
    async fn test_kafka_consumer_basic_functionality() {
        let config = KafkaConfig::new(
            "localhost:9092".to_string(),
            "test-topic".to_string(),
            "test-consumer".to_string(),
        );

        // Test consumer creation
        let consumer = KafkaConsumer::new(config).await;
        assert!(consumer.is_ok());
        let consumer = consumer.unwrap();
        assert!(consumer.is_initialized());

        // Test consumer stats
        let stats = consumer.get_stats().await;
        assert!(stats.is_ok());
        let stats = stats.unwrap();
        assert_eq!(stats.total_reads, 0);
        assert_eq!(stats.total_records, 0);
    }
}
