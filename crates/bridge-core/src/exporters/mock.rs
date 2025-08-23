//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Mock exporter for testing the OpenTelemetry Data Lake Bridge
//!
//! This module provides a mock exporter for testing and development purposes.

use crate::error::BridgeResult;
use crate::health::checker::HealthCheckCallback;
use crate::health::types::HealthCheckResult;
use crate::traits::exporter::LakehouseExporter;
use crate::types::{ExportResult, ProcessedBatch};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Mock exporter configuration
#[derive(Debug, Clone)]
pub struct MockExporterConfig {
    /// Whether to simulate export delays
    pub simulate_delay: bool,

    /// Export delay in milliseconds
    pub export_delay_ms: u64,

    /// Whether to simulate errors
    pub simulate_errors: bool,

    /// Error probability (0.0 to 1.0)
    pub error_probability: f64,

    /// Whether to simulate backpressure
    pub simulate_backpressure: bool,

    /// Backpressure threshold (records per second)
    pub backpressure_threshold: u64,

    /// Export destination
    pub destination: String,
}

impl Default for MockExporterConfig {
    fn default() -> Self {
        Self {
            simulate_delay: false,
            export_delay_ms: 100,
            simulate_errors: false,
            error_probability: 0.1,
            simulate_backpressure: false,
            backpressure_threshold: 1000,
            destination: "mock://localhost/test".to_string(),
        }
    }
}

/// Mock exporter for testing
pub struct MockExporter {
    /// Exporter configuration
    config: MockExporterConfig,

    /// Running state
    running: Arc<RwLock<bool>>,

    /// Statistics
    stats: Arc<RwLock<crate::traits::exporter::ExporterStats>>,

    /// Export counter
    export_counter: Arc<RwLock<u64>>,
}

impl MockExporter {
    /// Create a new mock exporter
    pub fn new(config: MockExporterConfig) -> Self {
        debug!(
            "Creating mock exporter with destination: {}",
            config.destination
        );

        Self {
            config,
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(crate::traits::exporter::ExporterStats {
                total_batches: 0,
                total_records: 0,
                batches_per_minute: 0,
                records_per_minute: 0,
                avg_export_time_ms: 0.0,
                error_count: 0,
                last_export_time: None,
            })),
            export_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Start the mock exporter
    pub async fn start(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(crate::error::BridgeError::internal(
                "Mock exporter is already running",
            ));
        }

        *running = true;
        debug!("Starting mock exporter");

        Ok(())
    }

    /// Stop the mock exporter
    pub async fn stop(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Mock exporter is not running",
            ));
        }

        *running = false;
        debug!("Stopping mock exporter");

        Ok(())
    }

    /// Simulate export delay
    async fn simulate_delay(&self) {
        if self.config.simulate_delay {
            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.config.export_delay_ms,
            ))
            .await;
        }
    }

    /// Check if we should simulate an error
    fn should_simulate_error(&self) -> bool {
        if !self.config.simulate_errors {
            return false;
        }

        let random_value = rand::random::<f64>();
        random_value < self.config.error_probability
    }

    /// Check if we should simulate backpressure
    fn should_simulate_backpressure(&self) -> bool {
        if !self.config.simulate_backpressure {
            return false;
        }

        // Simple backpressure simulation based on export counter
        if let Ok(counter) = self.export_counter.try_read() {
            *counter > self.config.backpressure_threshold
        } else {
            false
        }
    }
}

#[async_trait]
impl LakehouseExporter for MockExporter {
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        let running = self.running.read().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Mock exporter is not running",
            ));
        }

        let start_time = std::time::Instant::now();

        // Simulate export delay
        self.simulate_delay().await;

        // Check if we should simulate backpressure
        if self.should_simulate_backpressure() {
            return Err(crate::error::BridgeError::export(
                "Simulated backpressure from mock exporter",
            ));
        }

        // Check if we should simulate an error
        if self.should_simulate_error() {
            // Update error statistics
            {
                let mut stats = self.stats.write().await;
                stats.error_count += 1;
                stats.last_export_time = Some(chrono::Utc::now());
            }

            return Err(crate::error::BridgeError::export(
                "Simulated error from mock exporter",
            ));
        }

        // Simulate export
        let record_count = batch.records.len();
        let bytes_exported = record_count * 100; // Simulate 100 bytes per record

        // Update export counter
        {
            let mut counter = self.export_counter.write().await;
            *counter += record_count as u64;
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_batches += 1;
            stats.total_records += record_count as u64;
            stats.avg_export_time_ms = start_time.elapsed().as_millis() as f64;
            stats.last_export_time = Some(chrono::Utc::now());
        }

        debug!(
            "Mock exported batch: {} records, {} bytes in {:?}",
            record_count,
            bytes_exported,
            start_time.elapsed()
        );

        Ok(ExportResult {
            timestamp: chrono::Utc::now(),
            status: crate::types::ExportStatus::Success,
            records_exported: record_count,
            records_failed: 0,
            duration_ms: start_time.elapsed().as_millis() as u64,
            metadata: std::collections::HashMap::from([
                ("destination".to_string(), self.config.destination.clone()),
                ("bytes_exported".to_string(), bytes_exported.to_string()),
            ]),
            errors: vec![],
        })
    }

    fn name(&self) -> &str {
        "mock_exporter"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        let running = self.running.read().await;
        Ok(*running)
    }

    async fn get_stats(&self) -> BridgeResult<crate::traits::exporter::ExporterStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        self.stop().await
    }
}

#[async_trait]
impl HealthCheckCallback for MockExporter {
    async fn check(&self) -> BridgeResult<HealthCheckResult> {
        let running = self.running.read().await;

        Ok(HealthCheckResult {
            component: "mock_exporter".to_string(),
            status: if *running {
                crate::health::types::HealthStatus::Healthy
            } else {
                crate::health::types::HealthStatus::Unhealthy
            },
            timestamp: chrono::Utc::now(),
            duration_ms: 0,
            message: if *running {
                "Mock exporter is running".to_string()
            } else {
                "Mock exporter is not running".to_string()
            },
            details: None,
            errors: vec![],
        })
    }

    fn component_name(&self) -> &str {
        "mock_exporter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ProcessedRecord, ProcessingStatus};

    #[tokio::test]
    async fn test_mock_exporter_creation() {
        let config = MockExporterConfig::default();
        let exporter = MockExporter::new(config);

        let stats = exporter.get_stats().await.unwrap();
        assert_eq!(stats.total_records, 0);
    }

    #[tokio::test]
    async fn test_mock_exporter_start_stop() {
        let config = MockExporterConfig::default();
        let exporter = MockExporter::new(config);

        // Start the exporter
        let result = exporter.start().await;
        assert!(result.is_ok());

        // Check health
        let health = exporter.health_check().await.unwrap();
        assert!(health);

        // Stop the exporter
        let result = exporter.stop().await;
        assert!(result.is_ok());

        // Check health again
        let health = exporter.health_check().await.unwrap();
        assert!(!health);
    }

    #[tokio::test]
    async fn test_mock_exporter_export() {
        let config = MockExporterConfig::default();
        let exporter = MockExporter::new(config);
        exporter.start().await.unwrap();

        let batch = create_test_batch();
        let result = exporter.export(batch).await.unwrap();

        // Should export successfully
        assert_eq!(result.records_exported, 2);
        assert_eq!(result.status, crate::types::ExportStatus::Success);

        // Check statistics
        let stats = exporter.get_stats().await.unwrap();
        assert_eq!(stats.total_records, 2);
        assert_eq!(stats.total_batches, 1);

        exporter.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_exporter_with_delay() {
        let mut config = MockExporterConfig::default();
        config.simulate_delay = true;
        config.export_delay_ms = 50;

        let exporter = MockExporter::new(config);
        exporter.start().await.unwrap();

        let batch = create_test_batch();
        let start_time = std::time::Instant::now();
        let result = exporter.export(batch).await.unwrap();
        let export_time = start_time.elapsed();

        // Should have taken at least the delay time
        assert!(export_time.as_millis() >= 50);
        assert_eq!(result.records_exported, 2);

        exporter.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_exporter_with_errors() {
        let mut config = MockExporterConfig::default();
        config.simulate_errors = true;
        config.error_probability = 1.0; // Always error

        let exporter = MockExporter::new(config);
        exporter.start().await.unwrap();

        let batch = create_test_batch();
        let result = exporter.export(batch).await;

        // Should return an error
        assert!(result.is_err());

        exporter.stop().await.unwrap();
    }

    fn create_test_batch() -> ProcessedBatch {
        ProcessedBatch {
            original_batch_id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            status: ProcessingStatus::Success,
            records: vec![
                ProcessedRecord {
                    original_id: uuid::Uuid::new_v4(),
                    status: ProcessingStatus::Success,
                    transformed_data: Some(crate::types::TelemetryData::Metric(
                        crate::types::MetricData {
                            name: "test_metric".to_string(),
                            description: None,
                            unit: None,
                            metric_type: crate::types::MetricType::Counter,
                            value: crate::types::MetricValue::Counter(1.0),
                            labels: std::collections::HashMap::new(),
                            timestamp: chrono::Utc::now(),
                        },
                    )),
                    metadata: std::collections::HashMap::new(),
                    errors: vec![],
                },
                ProcessedRecord {
                    original_id: uuid::Uuid::new_v4(),
                    status: ProcessingStatus::Success,
                    transformed_data: Some(crate::types::TelemetryData::Metric(
                        crate::types::MetricData {
                            name: "test_metric2".to_string(),
                            description: None,
                            unit: None,
                            metric_type: crate::types::MetricType::Counter,
                            value: crate::types::MetricValue::Counter(2.0),
                            labels: std::collections::HashMap::new(),
                            timestamp: chrono::Utc::now(),
                        },
                    )),
                    metadata: std::collections::HashMap::new(),
                    errors: vec![],
                },
            ],
            metadata: std::collections::HashMap::new(),
            errors: vec![],
        }
    }
}
