//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Mock receiver for testing the OpenTelemetry Data Lake Bridge
//!
//! This module provides a mock receiver for testing and development purposes.

use crate::error::BridgeResult;
use crate::health::checker::HealthCheckCallback;
use crate::health::types::HealthCheckResult;
use crate::traits::TelemetryReceiver;
use crate::types::{TelemetryBatch, TelemetryRecord, TelemetryType};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Mock receiver configuration
#[derive(Debug, Clone)]
pub struct MockReceiverConfig {
    /// Whether to generate data automatically
    pub auto_generate: bool,

    /// Interval for auto-generation (in milliseconds)
    pub generation_interval_ms: u64,

    /// Number of records to generate per batch
    pub records_per_batch: usize,

    /// Whether to simulate errors
    pub simulate_errors: bool,

    /// Error probability (0.0 to 1.0)
    pub error_probability: f64,
}

impl Default for MockReceiverConfig {
    fn default() -> Self {
        Self {
            auto_generate: true,
            generation_interval_ms: 1000, // 1 second
            records_per_batch: 10,
            simulate_errors: false,
            error_probability: 0.1,
        }
    }
}

/// Mock receiver for testing
pub struct MockReceiver {
    /// Receiver configuration
    config: MockReceiverConfig,

    /// Generated data buffer
    buffer: Arc<RwLock<Vec<TelemetryBatch>>>,

    /// Running state
    running: Arc<RwLock<bool>>,

    /// Statistics
    stats: Arc<RwLock<crate::traits::ReceiverStats>>,

    /// Counter for generating unique IDs
    counter: Arc<RwLock<u64>>,
}

impl MockReceiver {
    /// Create a new mock receiver
    pub fn new(config: MockReceiverConfig) -> Self {
        debug!(
            "Creating mock receiver with auto_generate: {}",
            config.auto_generate
        );

        Self {
            config,
            buffer: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(crate::traits::ReceiverStats {
                total_records: 0,
                records_per_minute: 0,
                total_bytes: 0,
                bytes_per_minute: 0,
                error_count: 0,
                last_receive_time: None,
                protocol_stats: None,
            })),
            counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Start the mock receiver
    pub async fn start(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(crate::error::BridgeError::internal(
                "Mock receiver is already running",
            ));
        }

        *running = true;
        debug!("Starting mock receiver");

        if self.config.auto_generate {
            self.start_auto_generation().await?;
        }

        Ok(())
    }

    /// Stop the mock receiver
    pub async fn stop(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Mock receiver is not running",
            ));
        }

        *running = false;
        debug!("Stopping mock receiver");

        Ok(())
    }

    /// Start auto-generation of mock data
    async fn start_auto_generation(&self) -> BridgeResult<()> {
        let config = self.config.clone();
        let buffer = Arc::clone(&self.buffer);
        let running = Arc::clone(&self.running);
        let stats = Arc::clone(&self.stats);
        let counter = Arc::clone(&self.counter);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(
                config.generation_interval_ms,
            ));

            loop {
                interval.tick().await;

                // Check if still running
                let is_running = *running.read().await;
                if !is_running {
                    break;
                }

                // Generate mock batch
                let batch = Self::generate_mock_batch(&config, &counter).await;

                // Add to buffer
                {
                    let mut buffer_guard = buffer.write().await;
                    buffer_guard.push(batch.clone());
                }

                // Update statistics
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.total_records += batch.records.len() as u64;
                    stats_guard.total_bytes += batch.size as u64;
                    stats_guard.last_receive_time = Some(chrono::Utc::now());
                }

                debug!("Generated mock batch with {} records", batch.records.len());
            }
        });

        Ok(())
    }

    /// Generate a mock telemetry batch
    async fn generate_mock_batch(
        config: &MockReceiverConfig,
        counter: &Arc<RwLock<u64>>,
    ) -> TelemetryBatch {
        let mut counter_guard = counter.write().await;
        *counter_guard += 1;
        let batch_id = *counter_guard;

        let mut records = Vec::new();

        for i in 0..config.records_per_batch {
            let record = TelemetryRecord::new(
                TelemetryType::Metric,
                crate::types::TelemetryData::Metric(crate::types::MetricData {
                    name: format!("mock.metric.{}", i),
                    description: Some(format!("Mock metric {}", i)),
                    unit: Some("count".to_string()),
                    metric_type: crate::types::MetricType::Counter,
                    value: crate::types::MetricValue::Counter((batch_id as f64) + (i as f64)),
                    labels: HashMap::from([
                        ("batch_id".to_string(), batch_id.to_string()),
                        ("record_index".to_string(), i.to_string()),
                        ("source".to_string(), "mock".to_string()),
                    ]),
                    timestamp: chrono::Utc::now(),
                }),
            );

            records.push(record);
        }

        TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: "mock".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::from([
                ("batch_id".to_string(), batch_id.to_string()),
                ("source".to_string(), "mock".to_string()),
            ]),
        }
    }

    /// Manually add a batch to the buffer (for testing)
    pub async fn add_batch(&self, batch: TelemetryBatch) -> BridgeResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.push(batch);
        Ok(())
    }
}

#[async_trait]
impl TelemetryReceiver for MockReceiver {
    async fn receive(&self) -> BridgeResult<TelemetryBatch> {
        let running = self.running.read().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Mock receiver is not running",
            ));
        }

        // Check if we should simulate an error
        if self.config.simulate_errors {
            let random_value = rand::random::<f64>();
            if random_value < self.config.error_probability {
                // Update error statistics
                {
                    let mut stats = self.stats.write().await;
                    stats.error_count += 1;
                }

                return Err(crate::error::BridgeError::internal(
                    "Simulated error from mock receiver",
                ));
            }
        }

        // Get data from buffer
        {
            let mut buffer = self.buffer.write().await;
            if let Some(batch) = buffer.pop() {
                debug!("Returning mock batch with {} records", batch.records.len());
                return Ok(batch);
            }
        }

        // If no data in buffer, generate a new batch
        let batch = Self::generate_mock_batch(&self.config, &self.counter).await;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_records += batch.records.len() as u64;
            stats.total_bytes += batch.size as u64;
            stats.last_receive_time = Some(chrono::Utc::now());
        }

        Ok(batch)
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        let running = self.running.read().await;
        Ok(*running)
    }

    async fn get_stats(&self) -> BridgeResult<crate::traits::ReceiverStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        self.stop().await
    }
}

#[async_trait]
impl HealthCheckCallback for MockReceiver {
    async fn check(&self) -> BridgeResult<HealthCheckResult> {
        let running = self.running.read().await;

        Ok(HealthCheckResult {
            component: "mock_receiver".to_string(),
            status: if *running {
                crate::health::types::HealthStatus::Healthy
            } else {
                crate::health::types::HealthStatus::Unhealthy
            },
            timestamp: chrono::Utc::now(),
            duration_ms: 0,
            message: if *running {
                "Mock receiver is running".to_string()
            } else {
                "Mock receiver is not running".to_string()
            },
            details: None,
            errors: vec![],
        })
    }

    fn component_name(&self) -> &str {
        "mock_receiver"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_receiver_creation() {
        let config = MockReceiverConfig::default();
        let receiver = MockReceiver::new(config);

        let stats = receiver.get_stats().await.unwrap();
        assert_eq!(stats.total_records, 0);
        assert_eq!(stats.total_bytes, 0);
    }

    #[tokio::test]
    async fn test_mock_receiver_health_check() {
        let config = MockReceiverConfig::default();
        let receiver = MockReceiver::new(config);

        // Should not be healthy when not running
        let health = receiver.health_check().await.unwrap();
        assert!(!health);
    }

    #[tokio::test]
    async fn test_mock_receiver_start_stop() {
        let config = MockReceiverConfig::default();
        let receiver = MockReceiver::new(config);

        // Start the receiver
        let result = receiver.start().await;
        assert!(result.is_ok());

        // Check health
        let health = receiver.health_check().await.unwrap();
        assert!(health);

        // Stop the receiver
        let result = receiver.stop().await;
        assert!(result.is_ok());

        // Check health again
        let health = receiver.health_check().await.unwrap();
        assert!(!health);
    }

    #[tokio::test]
    async fn test_mock_receiver_data_generation() {
        let mut config = MockReceiverConfig::default();
        config.auto_generate = false; // Disable auto-generation for this test
        config.records_per_batch = 5;

        let receiver = MockReceiver::new(config);

        // Start the receiver
        receiver.start().await.unwrap();

        // Receive data
        let batch = receiver.receive().await.unwrap();
        assert_eq!(batch.records.len(), 5);
        assert_eq!(batch.source, "mock");

        // Check statistics
        let stats = receiver.get_stats().await.unwrap();
        assert_eq!(stats.total_records, 5);
        assert_eq!(stats.total_bytes, 5);

        // Stop the receiver
        receiver.stop().await.unwrap();
    }
}
