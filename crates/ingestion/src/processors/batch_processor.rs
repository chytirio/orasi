//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Batch processor implementation
//!
//! This module provides a batch processor for batching telemetry data
//! during processing to improve efficiency.

use async_trait::async_trait;
use bridge_core::{
    traits::ProcessorStats,
    types::{ProcessedRecord, TelemetryRecord},
    BridgeResult, ProcessedBatch, TelemetryBatch, TelemetryProcessor as BridgeTelemetryProcessor,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;
use zstd::stream::write::Encoder;

use super::{BaseProcessor, ProcessorConfig};

/// Batch processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProcessorConfig {
    /// Processor name
    pub name: String,

    /// Processor version
    pub version: String,

    /// Maximum batch size
    pub max_batch_size: usize,

    /// Maximum batch time in milliseconds
    pub max_batch_time_ms: u64,

    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,

    /// Enable compression
    pub enable_compression: bool,

    /// Compression level
    pub compression_level: u32,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl BatchProcessorConfig {
    /// Create new batch processor configuration
    pub fn new(max_batch_size: usize, max_batch_time_ms: u64) -> Self {
        Self {
            name: "batch".to_string(),
            version: "1.0.0".to_string(),
            max_batch_size,
            max_batch_time_ms,
            flush_interval_ms: 5000,
            enable_compression: true,
            compression_level: 6,
            additional_config: HashMap::new(),
        }
    }
}

#[async_trait]
impl ProcessorConfig for BatchProcessorConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.max_batch_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Batch processor max batch size cannot be 0",
            ));
        }

        if self.max_batch_time_ms == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Batch processor max batch time cannot be 0",
            ));
        }

        if self.flush_interval_ms == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Batch processor flush interval cannot be 0",
            ));
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Batch processor implementation
pub struct BatchProcessor {
    base: BaseProcessor,
    config: BatchProcessorConfig,
    current_batch: Arc<RwLock<Option<TelemetryBatch>>>,
    last_flush_time: Arc<RwLock<DateTime<Utc>>>,
}

impl BatchProcessor {
    /// Create new batch processor
    pub async fn new(config: &dyn ProcessorConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<BatchProcessorConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration("Invalid batch processor configuration")
            })?
            .clone();

        config.validate().await?;

        let base = BaseProcessor::new(config.name.clone(), config.version.clone());

        Ok(Self {
            base,
            config,
            current_batch: Arc::new(RwLock::new(None)),
            last_flush_time: Arc::new(RwLock::new(Utc::now())),
        })
    }

    /// Add record to current batch
    async fn add_to_batch(&self, record: TelemetryRecord) -> BridgeResult<()> {
        let mut current_batch = self.current_batch.write().await;

        match current_batch.as_mut() {
            Some(batch) => {
                batch.records.push(record);
                batch.size = batch.records.len();
            }
            None => {
                *current_batch = Some(TelemetryBatch {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    source: "batch-processor".to_string(),
                    size: 1,
                    records: vec![record],
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(())
    }

    /// Check if batch should be flushed
    async fn should_flush(&self) -> bool {
        let current_batch = self.current_batch.read().await;
        let last_flush = self.last_flush_time.read().await;

        // Check if batch is full
        if let Some(batch) = current_batch.as_ref() {
            if batch.size >= self.config.max_batch_size {
                return true;
            }
        }

        // Check if batch time exceeded
        let now = Utc::now();
        let time_since_flush = now.signed_duration_since(*last_flush);
        if time_since_flush.num_milliseconds() as u64 >= self.config.max_batch_time_ms {
            return true;
        }

        false
    }

    /// Flush current batch
    async fn flush_batch(&self) -> BridgeResult<Option<TelemetryBatch>> {
        let mut current_batch = self.current_batch.write().await;
        let mut last_flush = self.last_flush_time.write().await;

        if let Some(batch) = current_batch.take() {
            *last_flush = Utc::now();
            Ok(Some(batch))
        } else {
            Ok(None)
        }
    }

    /// Compress batch if enabled
    pub async fn compress_batch(&self, batch: TelemetryBatch) -> BridgeResult<TelemetryBatch> {
        if !self.config.enable_compression {
            return Ok(batch);
        }

        // Serialize the batch to JSON for compression
        let json_data = serde_json::to_vec(&batch).map_err(|e| {
            bridge_core::BridgeError::ingestion(format!("Failed to serialize batch: {}", e))
        })?;

        // Compress the serialized data using zstd
        let compression_level = self.config.compression_level.min(22) as i32; // zstd max level is 22
        let mut encoder = Encoder::new(Vec::new(), compression_level).map_err(|e| {
            bridge_core::BridgeError::ingestion(format!("Failed to create zstd encoder: {}", e))
        })?;

        encoder.write_all(&json_data).map_err(|e| {
            bridge_core::BridgeError::ingestion(format!("Failed to write data to encoder: {}", e))
        })?;

        let compressed_data = encoder.finish().map_err(|e| {
            bridge_core::BridgeError::ingestion(format!("Failed to finish compression: {}", e))
        })?;

        // Create a new batch with compressed metadata
        let mut compressed_batch = batch;
        compressed_batch
            .metadata
            .insert("compression".to_string(), "zstd".to_string());
        compressed_batch.metadata.insert(
            "compression_level".to_string(),
            compression_level.to_string(),
        );
        compressed_batch
            .metadata
            .insert("original_size".to_string(), json_data.len().to_string());
        compressed_batch.metadata.insert(
            "compressed_size".to_string(),
            compressed_data.len().to_string(),
        );
        compressed_batch.metadata.insert(
            "compression_ratio".to_string(),
            format!(
                "{:.2}",
                compressed_data.len() as f64 / json_data.len() as f64
            ),
        );

        info!(
            "Compressed batch {}: {} bytes -> {} bytes (ratio: {:.2})",
            compressed_batch.id,
            json_data.len(),
            compressed_data.len(),
            compressed_data.len() as f64 / json_data.len() as f64
        );

        Ok(compressed_batch)
    }

    /// Decompress batch if it was compressed
    async fn decompress_batch(&self, batch: TelemetryBatch) -> BridgeResult<TelemetryBatch> {
        // Check if the batch was compressed
        if !batch.metadata.contains_key("compression") {
            return Ok(batch);
        }

        // For now, we'll just return the batch as-is since we're not actually
        // storing the compressed data in the batch structure
        // In a real implementation, you might want to store the compressed data
        // separately and decompress it when needed

        warn!("Decompression requested but not implemented - batch data is not actually compressed in memory");
        Ok(batch)
    }
}

#[async_trait]
impl BridgeTelemetryProcessor for BatchProcessor {
    async fn process(&self, batch: TelemetryBatch) -> BridgeResult<ProcessedBatch> {
        let start_time = std::time::Instant::now();

        // Add all records from input batch to current batch
        for record in batch.records {
            self.add_to_batch(record).await?;
        }

        // Check if we should flush
        let should_flush = self.should_flush().await;

        if should_flush {
            if let Some(flushed_batch) = self.flush_batch().await? {
                let compressed_batch = self.compress_batch(flushed_batch).await?;

                let processing_time = start_time.elapsed();
                let processing_time_ms = processing_time.as_millis() as f64;

                // Update statistics
                self.base
                    .update_stats(1, compressed_batch.size, processing_time_ms)
                    .await;

                Ok(ProcessedBatch {
                    original_batch_id: batch.id,
                    timestamp: Utc::now(),
                    status: bridge_core::types::ProcessingStatus::Success,
                    records: compressed_batch
                        .records
                        .into_iter()
                        .map(|r| ProcessedRecord {
                            original_id: r.id,
                            status: bridge_core::types::ProcessingStatus::Success,
                            transformed_data: Some(r.data),
                            metadata: r.attributes,
                            errors: vec![],
                        })
                        .collect(),
                    metadata: compressed_batch.metadata,
                    errors: vec![],
                })
            } else {
                // No batch to flush, return empty processed batch
                Ok(ProcessedBatch {
                    original_batch_id: batch.id,
                    timestamp: Utc::now(),
                    status: bridge_core::types::ProcessingStatus::Success,
                    records: vec![],
                    metadata: HashMap::new(),
                    errors: vec![],
                })
            }
        } else {
            // No flush needed, return empty processed batch
            Ok(ProcessedBatch {
                original_batch_id: batch.id,
                timestamp: Utc::now(),
                status: bridge_core::types::ProcessingStatus::Success,
                records: vec![],
                metadata: HashMap::new(),
                errors: vec![],
            })
        }
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn version(&self) -> &str {
        &self.base.version
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        // Implement actual health check
        // Check if the processor is functioning correctly
        let stats = self.base.stats.read().await;

        // Check if we've processed any batches recently
        let is_healthy = if let Some(last_process_time) = stats.last_process_time {
            let time_since_last_process = Utc::now().signed_duration_since(last_process_time);
            // Consider healthy if we've processed something in the last 5 minutes
            time_since_last_process.num_minutes() < 5
        } else {
            // If no processing has happened yet, consider healthy
            true
        };

        Ok(is_healthy)
    }

    async fn get_stats(&self) -> BridgeResult<ProcessorStats> {
        let stats = self.base.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down batch processor");

        // Flush any remaining batch
        if let Some(flushed_batch) = self.flush_batch().await? {
            info!("Flushed final batch with {} records", flushed_batch.size);
        }

        info!("Batch processor shutdown completed");
        Ok(())
    }
}

impl BatchProcessor {
    /// Force flush current batch
    pub async fn force_flush(&self) -> BridgeResult<Option<TelemetryBatch>> {
        self.flush_batch().await
    }

    /// Get current batch size
    pub async fn get_current_batch_size(&self) -> usize {
        let current_batch = self.current_batch.read().await;
        current_batch.as_ref().map(|b| b.size).unwrap_or(0)
    }

    /// Get batch configuration
    pub fn get_config(&self) -> &BatchProcessorConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_core::types::{MetricData, MetricType, MetricValue, TelemetryData, TelemetryType};

    #[tokio::test]
    async fn test_batch_compression_enabled() {
        // Create a batch processor with compression enabled
        let config = BatchProcessorConfig {
            name: "test-batch".to_string(),
            version: "1.0.0".to_string(),
            max_batch_size: 100,
            max_batch_time_ms: 5000,
            flush_interval_ms: 1000,
            enable_compression: true,
            compression_level: 6,
            additional_config: HashMap::new(),
        };

        let processor = BatchProcessor::new(&config).await.unwrap();

        // Create a test batch
        let records = vec![TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "test_metric".to_string(),
                description: Some("Test metric".to_string()),
                unit: Some("count".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(42.0),
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
            source: "test".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::new(),
        };

        // Test compression
        let compressed_batch = processor.compress_batch(batch).await.unwrap();

        // Verify compression metadata was added
        assert!(compressed_batch.metadata.contains_key("compression"));
        assert_eq!(compressed_batch.metadata["compression"], "zstd");
        assert!(compressed_batch.metadata.contains_key("compression_level"));
        assert!(compressed_batch.metadata.contains_key("original_size"));
        assert!(compressed_batch.metadata.contains_key("compressed_size"));
        assert!(compressed_batch.metadata.contains_key("compression_ratio"));

        // Verify compression ratio is reasonable (should be less than 1.0 for compression)
        let ratio: f64 = compressed_batch.metadata["compression_ratio"]
            .parse()
            .unwrap();
        assert!(ratio <= 1.0);
    }

    #[tokio::test]
    async fn test_batch_compression_disabled() {
        // Create a batch processor with compression disabled
        let config = BatchProcessorConfig {
            name: "test-batch".to_string(),
            version: "1.0.0".to_string(),
            max_batch_size: 100,
            max_batch_time_ms: 5000,
            flush_interval_ms: 1000,
            enable_compression: false,
            compression_level: 6,
            additional_config: HashMap::new(),
        };

        let processor = BatchProcessor::new(&config).await.unwrap();

        // Create a test batch
        let records = vec![TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "test_metric".to_string(),
                description: Some("Test metric".to_string()),
                unit: Some("count".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(42.0),
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
            source: "test".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::new(),
        };

        // Test compression (should return original batch unchanged)
        let compressed_batch = processor.compress_batch(batch.clone()).await.unwrap();

        // Verify no compression metadata was added
        assert!(!compressed_batch.metadata.contains_key("compression"));
        assert_eq!(compressed_batch.id, batch.id);
        assert_eq!(compressed_batch.records.len(), batch.records.len());
    }
}
