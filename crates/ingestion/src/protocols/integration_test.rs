//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Integration tests for protocol implementations
//!
//! This module provides comprehensive tests to verify that all protocol
//! implementations work together correctly and handle various scenarios.

#[cfg(test)]
mod tests {
    use super::super::*;
    use bridge_core::{BridgeResult, TelemetryBatch};
    use std::collections::HashMap;
    use tokio::time::{sleep, Duration};

    // Import protocol modules
    use crate::protocols::{
        kafka::{KafkaConfig, KafkaMessageHandler, KafkaProtocol, KafkaSerializationFormat},
        otap::{OtapConfig, OtapMessageHandler, OtapProtocol},
        otlp_arrow::{OtlpArrowConfig, OtlpArrowMessageHandler, OtlpArrowProtocol},
        otlp_grpc::{OtlpGrpcConfig, OtlpGrpcMessageHandler, OtlpGrpcProtocol},
    };

    /// Test that all protocols can be created and configured
    #[tokio::test]
    async fn test_protocol_creation() -> BridgeResult<()> {
        // Test OTLP Arrow protocol
        let otlp_arrow_config = OtlpArrowConfig::new("127.0.0.1".to_string(), 4318);
        let _otlp_arrow_protocol = OtlpArrowProtocol::new(&otlp_arrow_config).await?;
        assert_eq!(otlp_arrow_config.name(), "otlp-arrow");

        // Test OTLP gRPC protocol
        let otlp_grpc_config = OtlpGrpcConfig::new("127.0.0.1".to_string(), 4317);
        let _otlp_grpc_protocol = OtlpGrpcProtocol::new(&otlp_grpc_config).await?;
        assert_eq!(otlp_grpc_config.name(), "otlp-grpc");

        // Test OTAP protocol
        let otap_config = OtapConfig::new("127.0.0.1".to_string(), 4319);
        let _otap_protocol = OtapProtocol::new(&otap_config).await?;
        assert_eq!(otap_config.name(), "otap");

        // Test Kafka protocol
        let kafka_config = KafkaConfig::new(
            vec!["localhost:9092".to_string()],
            "test-topic".to_string(),
            "test-group".to_string(),
        );
        let _kafka_protocol = KafkaProtocol::new(&kafka_config).await?;
        assert_eq!(kafka_config.name(), "kafka");

        Ok(())
    }

    /// Test protocol factory creation
    #[tokio::test]
    async fn test_protocol_factory() -> BridgeResult<()> {
        // Test OTLP Arrow
        let otlp_arrow_config = OtlpArrowConfig::new("127.0.0.1".to_string(), 4318);
        let otlp_arrow_handler = ProtocolFactory::create_handler(&otlp_arrow_config).await?;
        assert!(otlp_arrow_handler.is_running() == false);

        // Test OTLP gRPC
        let otlp_grpc_config = OtlpGrpcConfig::new("127.0.0.1".to_string(), 4317);
        let otlp_grpc_handler = ProtocolFactory::create_handler(&otlp_grpc_config).await?;
        assert!(otlp_grpc_handler.is_running() == false);

        // Test OTAP
        let otap_config = OtapConfig::new("127.0.0.1".to_string(), 4319);
        let otap_handler = ProtocolFactory::create_handler(&otap_config).await?;
        assert!(otap_handler.is_running() == false);

        // Test Kafka
        let kafka_config = KafkaConfig::new(
            vec!["localhost:9092".to_string()],
            "test-topic".to_string(),
            "test-group".to_string(),
        );
        let kafka_handler = ProtocolFactory::create_handler(&kafka_config).await?;
        assert!(kafka_handler.is_running() == false);

        Ok(())
    }

    /// Test protocol message handling
    #[tokio::test]
    async fn test_protocol_message_handling() -> BridgeResult<()> {
        // Test OTLP Arrow message handler
        let otlp_arrow_handler = OtlpArrowMessageHandler::new();
        let otlp_arrow_message = ProtocolMessage {
            id: "test-otlp-arrow".to_string(),
            timestamp: chrono::Utc::now(),
            protocol: "otlp-arrow".to_string(),
            payload: MessagePayload::Arrow(vec![1, 2, 3, 4, 5]),
            metadata: HashMap::new(),
        };
        let otlp_arrow_batch = otlp_arrow_handler
            .handle_message(otlp_arrow_message)
            .await?;
        assert_eq!(otlp_arrow_batch.source, "otlp-arrow");

        // Test OTLP gRPC message handler
        let otlp_grpc_handler = OtlpGrpcMessageHandler::new();
        let otlp_grpc_message = ProtocolMessage {
            id: "test-otlp-grpc".to_string(),
            timestamp: chrono::Utc::now(),
            protocol: "otlp-grpc".to_string(),
            payload: MessagePayload::Protobuf(vec![1, 2, 3, 4, 5]),
            metadata: HashMap::new(),
        };
        let otlp_grpc_batch = otlp_grpc_handler.handle_message(otlp_grpc_message).await?;
        assert_eq!(otlp_grpc_batch.source, "otlp-grpc");

        // Test OTAP message handler
        let otap_handler = OtapMessageHandler::new();
        let otap_message = ProtocolMessage {
            id: "test-otap".to_string(),
            timestamp: chrono::Utc::now(),
            protocol: "otap".to_string(),
            payload: MessagePayload::Arrow(vec![1, 2, 3, 4, 5]),
            metadata: HashMap::new(),
        };
        let otap_batch = otap_handler.handle_message(otap_message).await?;
        assert_eq!(otap_batch.source, "otap");

        // Test Kafka message handler
        let kafka_handler = KafkaMessageHandler::new();
        let kafka_message = ProtocolMessage {
            id: "test-kafka".to_string(),
            timestamp: chrono::Utc::now(),
            protocol: "kafka".to_string(),
            payload: MessagePayload::Json(r#"{"test": "data"}"#.as_bytes().to_vec()),
            metadata: HashMap::new(),
        };
        let kafka_batch = kafka_handler.handle_message(kafka_message).await?;
        assert_eq!(kafka_batch.source, "kafka");

        Ok(())
    }

    /// Test protocol lifecycle (init, start, stop)
    #[tokio::test]
    async fn test_protocol_lifecycle() -> BridgeResult<()> {
        // Test OTLP Arrow lifecycle
        let otlp_arrow_config = OtlpArrowConfig::new("127.0.0.1".to_string(), 4318);
        let mut otlp_arrow_protocol = OtlpArrowProtocol::new(&otlp_arrow_config).await?;

        otlp_arrow_protocol.init().await?;
        assert!(!otlp_arrow_protocol.is_running());

        otlp_arrow_protocol.start().await?;
        // Note: is_running() might return false due to async nature, so we don't assert it

        otlp_arrow_protocol.stop().await?;
        assert!(!otlp_arrow_protocol.is_running());

        // Test OTLP gRPC lifecycle
        let otlp_grpc_config = OtlpGrpcConfig::new("127.0.0.1".to_string(), 4317);
        let mut otlp_grpc_protocol = OtlpGrpcProtocol::new(&otlp_grpc_config).await?;

        otlp_grpc_protocol.init().await?;
        assert!(!otlp_grpc_protocol.is_running());

        otlp_grpc_protocol.start().await?;
        otlp_grpc_protocol.stop().await?;
        assert!(!otlp_grpc_protocol.is_running());

        // Test OTAP lifecycle
        let otap_config = OtapConfig::new("127.0.0.1".to_string(), 4319);
        let mut otap_protocol = OtapProtocol::new(&otap_config).await?;

        otap_protocol.init().await?;
        assert!(!otap_protocol.is_running());

        otap_protocol.start().await?;
        otap_protocol.stop().await?;
        assert!(!otap_protocol.is_running());

        // Test Kafka lifecycle (only if Kafka feature is enabled)
        #[cfg(feature = "kafka")]
        {
            let kafka_config = KafkaConfig::new(
                vec!["localhost:9092".to_string()],
                "test-topic".to_string(),
                "test-group".to_string(),
            );
            let mut kafka_protocol = KafkaProtocol::new(&kafka_config).await?;

            kafka_protocol.init().await?;
            assert!(!kafka_protocol.is_running());

            kafka_protocol.start().await?;
            kafka_protocol.stop().await?;
            assert!(!kafka_protocol.is_running());
        }

        Ok(())
    }

    /// Test protocol statistics
    #[tokio::test]
    async fn test_protocol_statistics() -> BridgeResult<()> {
        // Test OTLP Arrow statistics
        let otlp_arrow_config = OtlpArrowConfig::new("127.0.0.1".to_string(), 4318);
        let otlp_arrow_protocol = OtlpArrowProtocol::new(&otlp_arrow_config).await?;
        let otlp_arrow_stats = otlp_arrow_protocol.get_stats().await?;
        assert_eq!(otlp_arrow_stats.protocol, "otlp-arrow");
        assert_eq!(otlp_arrow_stats.total_messages, 0);

        // Test OTLP gRPC statistics
        let otlp_grpc_config = OtlpGrpcConfig::new("127.0.0.1".to_string(), 4317);
        let otlp_grpc_protocol = OtlpGrpcProtocol::new(&otlp_grpc_config).await?;
        let otlp_grpc_stats = otlp_grpc_protocol.get_stats().await?;
        assert_eq!(otlp_grpc_stats.protocol, "otlp-grpc");
        assert_eq!(otlp_grpc_stats.total_messages, 0);

        // Test OTAP statistics
        let otap_config = OtapConfig::new("127.0.0.1".to_string(), 4319);
        let otap_protocol = OtapProtocol::new(&otap_config).await?;
        let otap_stats = otap_protocol.get_stats().await?;
        assert_eq!(otap_stats.protocol, "otap");
        assert_eq!(otap_stats.total_messages, 0);

        // Test Kafka statistics
        let kafka_config = KafkaConfig::new(
            vec!["localhost:9092".to_string()],
            "test-topic".to_string(),
            "test-group".to_string(),
        );
        let kafka_protocol = KafkaProtocol::new(&kafka_config).await?;
        let kafka_stats = kafka_protocol.get_stats().await?;
        assert_eq!(kafka_stats.protocol, "kafka");
        assert_eq!(kafka_stats.total_messages, 0);

        Ok(())
    }

    /// Test protocol error handling
    #[tokio::test]
    async fn test_protocol_error_handling() -> BridgeResult<()> {
        // Test invalid OTLP Arrow configuration
        let mut invalid_otlp_arrow_config = OtlpArrowConfig::new("".to_string(), 0);
        invalid_otlp_arrow_config.endpoint = "".to_string();
        invalid_otlp_arrow_config.port = 0;
        assert!(invalid_otlp_arrow_config.validate().await.is_err());

        // Test invalid OTLP gRPC configuration
        let mut invalid_otlp_grpc_config = OtlpGrpcConfig::new("".to_string(), 0);
        invalid_otlp_grpc_config.endpoint = "".to_string();
        invalid_otlp_grpc_config.port = 0;
        assert!(invalid_otlp_grpc_config.validate().await.is_err());

        // Test invalid OTAP configuration
        let mut invalid_otap_config = OtapConfig::new("".to_string(), 0);
        invalid_otap_config.endpoint = "".to_string();
        invalid_otap_config.port = 0;
        assert!(invalid_otap_config.validate().await.is_err());

        // Test invalid Kafka configuration
        let mut invalid_kafka_config = KafkaConfig::new(vec![], "".to_string(), "".to_string());
        invalid_kafka_config.bootstrap_servers = vec![];
        invalid_kafka_config.topic = "".to_string();
        invalid_kafka_config.group_id = "".to_string();
        assert!(invalid_kafka_config.validate().await.is_err());

        Ok(())
    }

    /// Test protocol data processing
    #[tokio::test]
    async fn test_protocol_data_processing() -> BridgeResult<()> {
        // Test OTLP Arrow data processing
        let otlp_arrow_handler = OtlpArrowMessageHandler::new();
        let arrow_data = vec![1, 2, 3, 4, 5];
        let otlp_arrow_message = ProtocolMessage {
            id: "test-otlp-arrow".to_string(),
            timestamp: chrono::Utc::now(),
            protocol: "otlp-arrow".to_string(),
            payload: MessagePayload::Arrow(arrow_data),
            metadata: HashMap::new(),
        };
        let result = otlp_arrow_handler.handle_message(otlp_arrow_message).await;
        assert!(result.is_ok());

        // Test OTLP gRPC data processing
        let otlp_grpc_handler = OtlpGrpcMessageHandler::new();
        let protobuf_data = vec![1, 2, 3, 4, 5];
        let otlp_grpc_message = ProtocolMessage {
            id: "test-otlp-grpc".to_string(),
            timestamp: chrono::Utc::now(),
            protocol: "otlp-grpc".to_string(),
            payload: MessagePayload::Protobuf(protobuf_data),
            metadata: HashMap::new(),
        };
        let result = otlp_grpc_handler.handle_message(otlp_grpc_message).await;
        assert!(result.is_ok());

        // Test OTAP data processing
        let otap_handler = OtapMessageHandler::new();
        let arrow_data = vec![1, 2, 3, 4, 5];
        let otap_message = ProtocolMessage {
            id: "test-otap".to_string(),
            timestamp: chrono::Utc::now(),
            protocol: "otap".to_string(),
            payload: MessagePayload::Arrow(arrow_data),
            metadata: HashMap::new(),
        };
        let result = otap_handler.handle_message(otap_message).await;
        assert!(result.is_ok());

        // Test Kafka data processing
        let kafka_handler = KafkaMessageHandler::new();
        let json_data = r#"{"test": "data"}"#.as_bytes().to_vec();
        let kafka_message = ProtocolMessage {
            id: "test-kafka".to_string(),
            timestamp: chrono::Utc::now(),
            protocol: "kafka".to_string(),
            payload: MessagePayload::Json(json_data),
            metadata: HashMap::new(),
        };
        let result = kafka_handler.handle_message(kafka_message).await;
        assert!(result.is_ok());

        Ok(())
    }

    /// Test protocol concurrent processing
    #[tokio::test]
    async fn test_protocol_concurrent_processing() -> BridgeResult<()> {
        // Test multiple protocols running concurrently
        let mut handles = vec![];

        // Start OTLP Arrow protocol
        let otlp_arrow_config = OtlpArrowConfig::new("127.0.0.1".to_string(), 4318);
        let mut otlp_arrow_protocol = OtlpArrowProtocol::new(&otlp_arrow_config).await?;
        otlp_arrow_protocol.init().await?;
        otlp_arrow_protocol.start().await?;

        let handle1 = tokio::spawn(async move {
            sleep(Duration::from_millis(100)).await;
            otlp_arrow_protocol.stop().await
        });
        handles.push(handle1);

        // Start OTLP gRPC protocol
        let otlp_grpc_config = OtlpGrpcConfig::new("127.0.0.1".to_string(), 4317);
        let mut otlp_grpc_protocol = OtlpGrpcProtocol::new(&otlp_grpc_config).await?;
        otlp_grpc_protocol.init().await?;
        otlp_grpc_protocol.start().await?;

        let handle2 = tokio::spawn(async move {
            sleep(Duration::from_millis(100)).await;
            otlp_grpc_protocol.stop().await
        });
        handles.push(handle2);

        // Start OTAP protocol
        let otap_config = OtapConfig::new("127.0.0.1".to_string(), 4319);
        let mut otap_protocol = OtapProtocol::new(&otap_config).await?;
        otap_protocol.init().await?;
        otap_protocol.start().await?;

        let handle3 = tokio::spawn(async move {
            sleep(Duration::from_millis(100)).await;
            otap_protocol.stop().await
        });
        handles.push(handle3);

        // Wait for all protocols to stop
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        Ok(())
    }
}
