//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Kafka connector unit tests
//! 
//! This module contains unit tests for individual components
//! of the Kafka connector.

use crate::config::KafkaConfig;
use crate::connector::KafkaConnector;
use crate::producer::KafkaProducer;
use crate::consumer::KafkaConsumer;
use crate::topic::KafkaTopic;
use crate::error::{KafkaError, KafkaResult};
use crate::tests::utils;

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = KafkaConfig::new(
            "localhost:9092".to_string(),
            "test_topic".to_string(),
            "test_group".to_string(),
        );
        
        assert_eq!(config.topic_name(), "test_topic");
        assert_eq!(config.bootstrap_servers(), "localhost:9092");
        assert_eq!(config.group_id(), "test_group");
    }

    #[test]
    fn test_config_validation() {
        let config = KafkaConfig::new(
            "localhost:9092".to_string(),
            "test_topic".to_string(),
            "test_group".to_string(),
        );
        
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_validation_empty_bootstrap_servers() {
        let mut config = KafkaConfig::new(
            "".to_string(),
            "test_topic".to_string(),
            "test_group".to_string(),
        );
        
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_empty_topic() {
        let mut config = KafkaConfig::new(
            "localhost:9092".to_string(),
            "".to_string(),
            "test_group".to_string(),
        );
        
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_empty_group_id() {
        let mut config = KafkaConfig::new(
            "localhost:9092".to_string(),
            "test_topic".to_string(),
            "".to_string(),
        );
        
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_default() {
        let config = KafkaConfig::default();
        assert!(!config.bootstrap_servers().is_empty());
        assert!(!config.topic_name().is_empty());
        assert!(!config.group_id().is_empty());
    }

    #[test]
    fn test_producer_config() {
        let config = KafkaConfig::new(
            "localhost:9092".to_string(),
            "test_topic".to_string(),
            "test_group".to_string(),
        );
        
        let producer_config = config.producer_config();
        assert!(producer_config.contains_key("bootstrap.servers"));
        assert!(producer_config.contains_key("client.id"));
        assert!(producer_config.contains_key("batch.size"));
    }

    #[test]
    fn test_consumer_config() {
        let config = KafkaConfig::new(
            "localhost:9092".to_string(),
            "test_topic".to_string(),
            "test_group".to_string(),
        );
        
        let consumer_config = config.consumer_config();
        assert!(consumer_config.contains_key("bootstrap.servers"));
        assert!(consumer_config.contains_key("group.id"));
        assert!(consumer_config.contains_key("auto.offset.reset"));
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = KafkaError::configuration("Test error");
        assert_eq!(error.category(), "configuration");
        assert!(error.error_code().is_some());
    }

    #[test]
    fn test_error_retryable() {
        let error = KafkaError::connection("Connection failed");
        assert!(error.is_retryable());
        
        let error = KafkaError::configuration("Config error");
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_error_transient() {
        let error = KafkaError::timeout("Timeout");
        assert!(error.is_transient());
        
        let error = KafkaError::validation("Validation error");
        assert!(!error.is_transient());
    }

    #[test]
    fn test_error_with_source() {
        let source_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let error = KafkaError::with_source("IO error", source_error);
        assert_eq!(error.category(), "wrapped");
    }
}

#[cfg(test)]
mod topic_tests {
    use super::*;

    #[test]
    fn test_topic_creation() {
        let topic_config = crate::topic::KafkaTopicConfig {
            name: "test_topic".to_string(),
            partitions: 3,
            replication_factor: 1,
            config: std::collections::HashMap::new(),
        };

        let topic = KafkaTopic::new(topic_config);
        assert_eq!(topic.config().name, "test_topic");
        assert_eq!(topic.config().partitions, 3);
        assert_eq!(topic.config().replication_factor, 1);
    }

    #[tokio::test]
    async fn test_topic_initialization() {
        let topic_config = crate::topic::KafkaTopicConfig {
            name: "test_topic".to_string(),
            partitions: 3,
            replication_factor: 1,
            config: std::collections::HashMap::new(),
        };

        let mut topic = KafkaTopic::new(topic_config);
        let result = topic.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_topic_manager() {
        let mut manager = crate::topic::KafkaTopicManager::new();
        let config = utils::create_test_config();
        
        let result = manager.create_topic(config).await;
        assert!(result.is_ok());
        
        let topics = manager.list_topics();
        assert!(!topics.is_empty());
    }
}

#[cfg(test)]
mod producer_tests {
    use super::*;
    use bridge_core::types::{MetricsBatch, MetricsRecord};

    #[tokio::test]
    async fn test_producer_creation() {
        let config = utils::create_test_config();
        let producer = KafkaProducer::new(config).await;
        assert!(producer.is_ok());
    }

    #[tokio::test]
    async fn test_producer_metrics() {
        let config = utils::create_test_config();
        let producer = KafkaProducer::new(config).await.unwrap();
        
        let batch = MetricsBatch {
            records: vec![
                MetricsRecord {
                    name: "test_metric".to_string(),
                    value: 42.0,
                    timestamp: chrono::Utc::now(),
                    labels: std::collections::HashMap::new(),
                }
            ],
        };
        
        let result = producer.write_metrics(batch).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_producer_health_check() {
        let config = utils::create_test_config();
        let producer = KafkaProducer::new(config).await.unwrap();
        
        let health = producer.health_check().await;
        assert!(health.is_ok());
    }

    #[tokio::test]
    async fn test_producer_stats() {
        let config = utils::create_test_config();
        let producer = KafkaProducer::new(config).await.unwrap();
        
        let stats = producer.get_stats().await;
        assert!(stats.is_ok());
    }
}

#[cfg(test)]
mod consumer_tests {
    use super::*;
    use bridge_core::types::{MetricsQuery, TracesQuery, LogsQuery};

    #[tokio::test]
    async fn test_consumer_creation() {
        let config = utils::create_test_config();
        let consumer = KafkaConsumer::new(config).await;
        assert!(consumer.is_ok());
    }

    #[tokio::test]
    async fn test_consumer_metrics_query() {
        let config = utils::create_test_config();
        let consumer = KafkaConsumer::new(config).await.unwrap();
        
        let query = MetricsQuery {
            name: Some("test_metric".to_string()),
            start_time: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
            end_time: Some(chrono::Utc::now()),
            labels: std::collections::HashMap::new(),
            limit: Some(100),
        };
        
        let result = consumer.query_metrics(query).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_consumer_traces_query() {
        let config = utils::create_test_config();
        let consumer = KafkaConsumer::new(config).await.unwrap();
        
        let query = TracesQuery {
            trace_id: Some("test_trace".to_string()),
            start_time: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
            end_time: Some(chrono::Utc::now()),
            service_name: Some("test_service".to_string()),
            limit: Some(100),
        };
        
        let result = consumer.query_traces(query).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_consumer_logs_query() {
        let config = utils::create_test_config();
        let consumer = KafkaConsumer::new(config).await.unwrap();
        
        let query = LogsQuery {
            start_time: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
            end_time: Some(chrono::Utc::now()),
            service_name: Some("test_service".to_string()),
            level: Some("INFO".to_string()),
            limit: Some(100),
        };
        
        let result = consumer.query_logs(query).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_consumer_custom_query() {
        let config = utils::create_test_config();
        let consumer = KafkaConsumer::new(config).await.unwrap();
        
        let query = "SELECT * FROM telemetry WHERE timestamp > '2023-01-01'".to_string();
        let result = consumer.execute_query(query).await;
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod connector_tests {
    use super::*;

    #[tokio::test]
    async fn test_connector_creation() {
        let connector = utils::create_test_connector().await;
        assert!(connector.is_ok());
    }

    #[tokio::test]
    async fn test_connector_producer() {
        let connector = utils::create_test_connector().await.unwrap();
        let producer = connector.writer().await;
        assert!(producer.is_ok());
    }

    #[tokio::test]
    async fn test_connector_consumer() {
        let connector = utils::create_test_connector().await.unwrap();
        let consumer = connector.reader().await;
        assert!(consumer.is_ok());
    }

    #[tokio::test]
    async fn test_connector_name_version() {
        let connector = utils::create_test_connector().await.unwrap();
        assert_eq!(connector.name(), "kafka-connector");
        assert!(!connector.version().is_empty());
    }

    #[tokio::test]
    async fn test_connector_shutdown() {
        let connector = utils::create_test_connector().await.unwrap();
        let result = connector.shutdown().await;
        assert!(result.is_ok());
    }
}
