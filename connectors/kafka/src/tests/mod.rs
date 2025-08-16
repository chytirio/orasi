//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Kafka connector tests
//! 
//! This module contains comprehensive unit and integration tests
//! for the Kafka connector functionality.

pub mod unit;
pub mod integration;
pub mod common;

use crate::config::KafkaConfig;
use crate::connector::KafkaConnector;
use crate::error::KafkaResult;

/// Test utilities and helpers
pub mod utils {
    use super::*;
    use std::env;

    /// Create a test configuration
    pub fn create_test_config() -> KafkaConfig {
        // Use environment variables for test configuration
        let bootstrap_servers = env::var("KAFKA_TEST_BOOTSTRAP_SERVERS")
            .unwrap_or_else(|_| "localhost:9092".to_string());
        let topic_name = env::var("KAFKA_TEST_TOPIC")
            .unwrap_or_else(|_| "test_telemetry".to_string());
        let group_id = env::var("KAFKA_TEST_GROUP_ID")
            .unwrap_or_else(|_| "test-consumer-group".to_string());
        
        KafkaConfig::new(
            bootstrap_servers,
            topic_name,
            group_id,
        )
    }

    /// Create a test connector
    pub async fn create_test_connector() -> KafkaResult<KafkaConnector> {
        let config = create_test_config();
        KafkaConnector::connect(config).await
    }

    /// Check if Kafka is available for testing
    pub fn is_kafka_available() -> bool {
        env::var("KAFKA_TEST_BOOTSTRAP_SERVERS").is_ok() || 
        env::var("SKIP_KAFKA_TESTS").is_err()
    }

    /// Skip test if Kafka is not available
    pub fn skip_if_no_kafka() {
        if !is_kafka_available() {
            eprintln!("Skipping Kafka test - no Kafka instance available");
            std::process::exit(0);
        }
    }

    /// Create a unique test topic name
    pub fn create_unique_topic_name() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("test_telemetry_{}", timestamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connector_creation() {
        utils::skip_if_no_kafka();
        let connector = utils::create_test_connector().await;
        assert!(connector.is_ok());
    }

    #[tokio::test]
    async fn test_connector_health_check() {
        utils::skip_if_no_kafka();
        let connector = utils::create_test_connector().await.unwrap();
        let health = connector.health_check().await;
        assert!(health.is_ok());
    }

    #[tokio::test]
    async fn test_connector_stats() {
        utils::skip_if_no_kafka();
        let connector = utils::create_test_connector().await.unwrap();
        let stats = connector.get_stats().await;
        assert!(stats.is_ok());
    }
}
