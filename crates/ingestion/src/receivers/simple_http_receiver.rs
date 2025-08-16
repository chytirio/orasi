//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Simple HTTP receiver implementation
//!
//! This module provides a simple HTTP receiver that can handle telemetry data
//! over HTTP endpoints.

use async_trait::async_trait;
use bridge_core::{
    traits::ReceiverStats,
    types::{MetricData, MetricValue, TelemetryData, TelemetryRecord, TelemetryType},
    BridgeResult, TelemetryBatch, TelemetryReceiver,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

/// Simple HTTP receiver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleHttpReceiverConfig {
    /// Receiver name
    pub name: String,

    /// Receiver version
    pub version: String,

    /// HTTP endpoint to listen on
    pub endpoint: String,

    /// Port to listen on
    pub port: u16,

    /// Batch size for processing
    pub batch_size: usize,
}

impl SimpleHttpReceiverConfig {
    /// Create new simple HTTP receiver configuration
    pub fn new(endpoint: String, port: u16) -> Self {
        Self {
            name: "simple-http".to_string(),
            version: "1.0.0".to_string(),
            endpoint,
            port,
            batch_size: 1000,
        }
    }
}

/// Simple HTTP receiver implementation
pub struct SimpleHttpReceiver {
    config: SimpleHttpReceiverConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ReceiverStats>>,
}

impl SimpleHttpReceiver {
    /// Create new simple HTTP receiver
    pub fn new(config: SimpleHttpReceiverConfig) -> Self {
        let stats = ReceiverStats {
            total_records: 0,
            records_per_minute: 0,
            total_bytes: 0,
            bytes_per_minute: 0,
            error_count: 0,
            last_receive_time: None,
            protocol_stats: None,
        };

        Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
        }
    }

    /// Start the receiver
    pub async fn start(&mut self) -> BridgeResult<()> {
        info!(
            "Starting simple HTTP receiver on {}:{}",
            self.config.endpoint, self.config.port
        );

        let mut is_running = self.is_running.write().await;
        *is_running = true;

        info!("Simple HTTP receiver started");
        Ok(())
    }

    /// Stop the receiver
    pub async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping simple HTTP receiver");

        let mut is_running = self.is_running.write().await;
        *is_running = false;

        info!("Simple HTTP receiver stopped");
        Ok(())
    }

    /// Update statistics
    async fn update_stats(&self, records: usize, bytes: usize) {
        let mut stats = self.stats.write().await;
        stats.total_records += records as u64;
        stats.total_bytes += bytes as u64;
        stats.last_receive_time = Some(Utc::now());
    }

    /// Increment error count
    async fn increment_error_count(&self) {
        let mut stats = self.stats.write().await;
        stats.error_count += 1;
    }
}

#[async_trait]
impl TelemetryReceiver for SimpleHttpReceiver {
    async fn receive(&self) -> BridgeResult<TelemetryBatch> {
        // For now, return a placeholder batch
        // In a real implementation, this would receive data from HTTP requests

        let records = vec![TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "simple_http_receiver_metric".to_string(),
                description: Some("Placeholder metric from simple HTTP receiver".to_string()),
                unit: Some("count".to_string()),
                metric_type: bridge_core::types::MetricType::Gauge,
                value: MetricValue::Gauge(1.0),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        }];

        let batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "simple-http".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::new(),
        };

        // Update statistics
        self.update_stats(batch.size, 0).await;

        Ok(batch)
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        let is_running = self.is_running.read().await;
        Ok(*is_running)
    }

    async fn get_stats(&self) -> BridgeResult<ReceiverStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down simple HTTP receiver");

        let mut is_running = self.is_running.write().await;
        *is_running = false;

        info!("Simple HTTP receiver shutdown completed");
        Ok(())
    }
}

impl SimpleHttpReceiver {
    /// Get configuration
    pub fn get_config(&self) -> &SimpleHttpReceiverConfig {
        &self.config
    }

    /// Get endpoint
    pub fn get_endpoint(&self) -> &str {
        &self.config.endpoint
    }

    /// Get port
    pub fn get_port(&self) -> u16 {
        self.config.port
    }

    /// Check if receiver is running
    pub async fn is_running(&self) -> bool {
        let is_running = self.is_running.read().await;
        *is_running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_http_receiver_creation() {
        let config = SimpleHttpReceiverConfig::new("127.0.0.1".to_string(), 8080);
        let receiver = SimpleHttpReceiver::new(config);

        assert_eq!(receiver.get_endpoint(), "127.0.0.1");
        assert_eq!(receiver.get_port(), 8080);
    }

    #[tokio::test]
    async fn test_simple_http_receiver_lifecycle() {
        let config = SimpleHttpReceiverConfig::new("127.0.0.1".to_string(), 8080);
        let mut receiver = SimpleHttpReceiver::new(config);

        // Initially not running
        assert!(!receiver.is_running().await);

        // Start receiver
        receiver.start().await.unwrap();
        assert!(receiver.is_running().await);

        // Health check should pass
        assert!(receiver.health_check().await.unwrap());

        // Stop receiver
        receiver.stop().await.unwrap();
        assert!(!receiver.is_running().await);
    }

    #[tokio::test]
    async fn test_simple_http_receiver_receive() {
        let config = SimpleHttpReceiverConfig::new("127.0.0.1".to_string(), 8080);
        let mut receiver = SimpleHttpReceiver::new(config);

        // Start receiver
        receiver.start().await.unwrap();

        // Receive data
        let batch = receiver.receive().await.unwrap();
        assert_eq!(batch.source, "simple-http");
        assert_eq!(batch.size, 1);
        assert_eq!(batch.records.len(), 1);

        // Check the record
        let record = &batch.records[0];
        assert_eq!(record.record_type, TelemetryType::Metric);

        if let TelemetryData::Metric(metric) = &record.data {
            assert_eq!(metric.name, "simple_http_receiver_metric");
            assert_eq!(
                metric.description,
                Some("Placeholder metric from simple HTTP receiver".to_string())
            );
        } else {
            panic!("Expected metric data");
        }
    }

    #[tokio::test]
    async fn test_simple_http_receiver_stats() {
        let config = SimpleHttpReceiverConfig::new("127.0.0.1".to_string(), 8080);
        let mut receiver = SimpleHttpReceiver::new(config);

        // Start receiver
        receiver.start().await.unwrap();

        // Receive data to update stats
        receiver.receive().await.unwrap();

        // Check stats
        let stats = receiver.get_stats().await.unwrap();
        assert_eq!(stats.total_records, 1);
        assert!(stats.last_receive_time.is_some());
    }
}
