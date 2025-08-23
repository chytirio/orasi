//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Mock processor for testing the OpenTelemetry Data Lake Bridge
//!
//! This module provides a mock processor for testing and development purposes.

use crate::error::BridgeResult;
use crate::health::checker::HealthCheckCallback;
use crate::health::types::HealthCheckResult;
use crate::traits::processor::TelemetryProcessor;
use crate::types::{ProcessedBatch, ProcessedRecord, ProcessingStatus, TelemetryBatch};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Mock processor configuration
#[derive(Debug, Clone)]
pub struct MockProcessorConfig {
    /// Whether to simulate processing delays
    pub simulate_delay: bool,

    /// Processing delay in milliseconds
    pub processing_delay_ms: u64,

    /// Whether to simulate errors
    pub simulate_errors: bool,

    /// Error probability (0.0 to 1.0)
    pub error_probability: f64,

    /// Whether to transform data
    pub transform_data: bool,

    /// Whether to add metadata
    pub add_metadata: bool,
}

impl Default for MockProcessorConfig {
    fn default() -> Self {
        Self {
            simulate_delay: false,
            processing_delay_ms: 100,
            simulate_errors: false,
            error_probability: 0.1,
            transform_data: false,
            add_metadata: true,
        }
    }
}

/// Mock processor for testing
pub struct MockProcessor {
    /// Processor configuration
    config: MockProcessorConfig,

    /// Running state
    running: Arc<RwLock<bool>>,

    /// Statistics
    stats: Arc<RwLock<crate::traits::processor::ProcessorStats>>,

    /// Counter for generating unique IDs
    counter: Arc<RwLock<u64>>,
}

impl MockProcessor {
    /// Create a new mock processor
    pub fn new(config: MockProcessorConfig) -> Self {
        debug!(
            "Creating mock processor with delay: {}, errors: {}",
            config.simulate_delay, config.simulate_errors
        );

        Self {
            config,
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(crate::traits::processor::ProcessorStats {
                total_batches: 0,
                total_records: 0,
                batches_per_minute: 0,
                records_per_minute: 0,
                avg_processing_time_ms: 0.0,
                error_count: 0,
                last_process_time: None,
            })),
            counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Start the mock processor
    pub async fn start(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(crate::error::BridgeError::internal(
                "Mock processor is already running",
            ));
        }

        *running = true;
        debug!("Starting mock processor");

        Ok(())
    }

    /// Stop the mock processor
    pub async fn stop(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Mock processor is not running",
            ));
        }

        *running = false;
        debug!("Stopping mock processor");

        Ok(())
    }

    /// Simulate processing delay
    async fn simulate_delay(&self) {
        if self.config.simulate_delay {
            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.config.processing_delay_ms,
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

    /// Transform a record (add metadata, etc.)
    fn transform_record(
        &self,
        record: &crate::types::TelemetryRecord,
    ) -> crate::types::TelemetryData {
        if !self.config.transform_data {
            return record.data.clone();
        }

        // For now, just return the original data
        // In a real implementation, this would transform the data
        record.data.clone()
    }
}

#[async_trait]
impl TelemetryProcessor for MockProcessor {
    async fn process(&self, batch: TelemetryBatch) -> BridgeResult<ProcessedBatch> {
        let running = self.running.read().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Mock processor is not running",
            ));
        }

        let start_time = std::time::Instant::now();

        // Simulate processing delay
        self.simulate_delay().await;

        // Check if we should simulate an error
        if self.should_simulate_error() {
            // Update error statistics
            {
                let mut stats = self.stats.write().await;
                stats.error_count += 1;
                stats.last_process_time = Some(chrono::Utc::now());
            }

            return Err(crate::error::BridgeError::internal(
                "Simulated error from mock processor",
            ));
        }

        // Process records
        let mut processed_records = Vec::new();

        let record_count = batch.records.len();
        for record in &batch.records {
            let transformed_data = self.transform_record(&record);

            let mut metadata = record.attributes.clone();
            if self.config.add_metadata {
                metadata.insert("processed_by".to_string(), "mock_processor".to_string());
                metadata.insert(
                    "processing_timestamp".to_string(),
                    chrono::Utc::now().to_rfc3339(),
                );
            }

            processed_records.push(ProcessedRecord {
                original_id: record.id,
                status: ProcessingStatus::Success,
                transformed_data: Some(transformed_data),
                metadata,
                errors: vec![],
            });
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_batches += 1;
            stats.total_records += record_count as u64;
            stats.avg_processing_time_ms = start_time.elapsed().as_millis() as f64;
            stats.last_process_time = Some(chrono::Utc::now());
        }

        debug!(
            "Mock processed batch: {} records in {:?}",
            processed_records.len(),
            start_time.elapsed()
        );

        Ok(ProcessedBatch {
            original_batch_id: batch.id,
            timestamp: batch.timestamp,
            status: ProcessingStatus::Success,
            records: processed_records,
            metadata: batch.metadata,
            errors: vec![],
        })
    }

    fn name(&self) -> &str {
        "mock_processor"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        let running = self.running.read().await;
        Ok(*running)
    }

    async fn get_stats(&self) -> BridgeResult<crate::traits::processor::ProcessorStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        self.stop().await
    }
}

#[async_trait]
impl HealthCheckCallback for MockProcessor {
    async fn check(&self) -> BridgeResult<HealthCheckResult> {
        let running = self.running.read().await;

        Ok(HealthCheckResult {
            component: "mock_processor".to_string(),
            status: if *running {
                crate::health::types::HealthStatus::Healthy
            } else {
                crate::health::types::HealthStatus::Unhealthy
            },
            timestamp: chrono::Utc::now(),
            duration_ms: 0,
            message: if *running {
                "Mock processor is running".to_string()
            } else {
                "Mock processor is not running".to_string()
            },
            details: None,
            errors: vec![],
        })
    }

    fn component_name(&self) -> &str {
        "mock_processor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MetricData, MetricType, MetricValue, TelemetryData, TelemetryType};

    #[tokio::test]
    async fn test_mock_processor_creation() {
        let config = MockProcessorConfig::default();
        let processor = MockProcessor::new(config);

        let stats = processor.get_stats().await.unwrap();
        assert_eq!(stats.total_records, 0);
    }

    #[tokio::test]
    async fn test_mock_processor_start_stop() {
        let config = MockProcessorConfig::default();
        let processor = MockProcessor::new(config);

        // Start the processor
        let result = processor.start().await;
        assert!(result.is_ok());

        // Check health
        let health = processor.health_check().await.unwrap();
        assert!(health);

        // Stop the processor
        let result = processor.stop().await;
        assert!(result.is_ok());

        // Check health again
        let health = processor.health_check().await.unwrap();
        assert!(!health);
    }

    #[tokio::test]
    async fn test_mock_processor_processing() {
        let config = MockProcessorConfig::default();
        let processor = MockProcessor::new(config);
        processor.start().await.unwrap();

        let batch = create_test_batch();
        let result = processor.process(batch).await.unwrap();

        // Should process all records
        assert_eq!(result.records.len(), 2);
        assert_eq!(result.status, ProcessingStatus::Success);

        // Check that metadata was added
        let first_record = &result.records[0];
        assert!(first_record.metadata.contains_key("processed_by"));

        processor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_processor_with_delay() {
        let mut config = MockProcessorConfig::default();
        config.simulate_delay = true;
        config.processing_delay_ms = 50;

        let processor = MockProcessor::new(config);
        processor.start().await.unwrap();

        let batch = create_test_batch();
        let start_time = std::time::Instant::now();
        let result = processor.process(batch).await.unwrap();
        let processing_time = start_time.elapsed();

        // Should have taken at least the delay time
        assert!(processing_time.as_millis() >= 50);
        assert_eq!(result.records.len(), 2);

        processor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_processor_with_errors() {
        let mut config = MockProcessorConfig::default();
        config.simulate_errors = true;
        config.error_probability = 1.0; // Always error

        let processor = MockProcessor::new(config);
        processor.start().await.unwrap();

        let batch = create_test_batch();
        let result = processor.process(batch).await;

        // Should return an error
        assert!(result.is_err());

        processor.stop().await.unwrap();
    }

    fn create_test_batch() -> TelemetryBatch {
        let record1 = crate::types::TelemetryRecord::new(
            crate::types::TelemetryType::Metric,
            crate::types::TelemetryData::Metric(crate::types::MetricData {
                name: "test_metric".to_string(),
                description: None,
                unit: None,
                metric_type: crate::types::MetricType::Counter,
                value: crate::types::MetricValue::Counter(1.0),
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            }),
        );

        let record2 = crate::types::TelemetryRecord::new(
            crate::types::TelemetryType::Metric,
            crate::types::TelemetryData::Metric(crate::types::MetricData {
                name: "test_metric2".to_string(),
                description: None,
                unit: None,
                metric_type: crate::types::MetricType::Counter,
                value: crate::types::MetricValue::Counter(2.0),
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            }),
        );

        crate::types::TelemetryBatch::new("test_source".to_string(), vec![record1, record2])
    }
}
