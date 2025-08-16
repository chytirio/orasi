//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! S3/Parquet exporter implementation
//! 
//! This module provides the S3/Parquet exporter that implements
//! the LakehouseExporter trait for exporting telemetry data to S3 in Parquet format.

use std::sync::Arc;
use std::time::Instant;
use async_trait::async_trait;
use tracing::{debug, error, info, warn};
use bridge_core::traits::LakehouseExporter;
use bridge_core::types::{TelemetryBatch, ExportResult};
use bridge_core::error::BridgeResult;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

use crate::config::S3ParquetConfig;
use crate::error::{S3ParquetError, S3ParquetResult};
use crate::writer::S3ParquetWriter;

/// S3/Parquet exporter implementation
pub struct S3ParquetExporter {
    /// S3/Parquet configuration
    config: S3ParquetConfig,
    /// Writer instance
    writer: Arc<S3ParquetWriter>,
    /// Exporter state
    initialized: bool,
    /// Statistics
    stats: Arc<RwLock<ExporterStats>>,
}

/// Exporter statistics
#[derive(Debug, Clone)]
struct ExporterStats {
    /// Total batches exported
    total_batches: u64,
    /// Total records exported
    total_records: u64,
    /// Batches per minute
    batches_per_minute: u64,
    /// Records per minute
    records_per_minute: u64,
    /// Average export time in milliseconds
    avg_export_time_ms: f64,
    /// Error count
    error_count: u64,
    /// Last export time
    last_export_time: Option<DateTime<Utc>>,
}

impl S3ParquetExporter {
    /// Create a new S3/Parquet exporter
    pub async fn new(config: S3ParquetConfig) -> S3ParquetResult<Self> {
        info!("Creating S3/Parquet exporter for bucket: {}", config.bucket());
        
        let writer = Arc::new(S3ParquetWriter::new(config.clone()).await?);
        
        let exporter = Self {
            config,
            writer,
            initialized: false,
            stats: Arc::new(RwLock::new(ExporterStats::default())),
        };
        
        exporter.initialize().await?;
        Ok(exporter)
    }

    /// Initialize the exporter
    async fn initialize(&self) -> S3ParquetResult<()> {
        debug!("Initializing S3/Parquet exporter");
        
        // Test writer connection
        self.writer.health_check().await?;
        
        info!("S3/Parquet exporter initialized successfully");
        Ok(())
    }

    /// Update exporter statistics
    async fn update_stats(&self, export_time_ms: u64, record_count: usize) {
        let mut stats = self.stats.write().await;
        stats.total_batches += 1;
        stats.total_records += record_count as u64;
        stats.last_export_time = Some(chrono::Utc::now());
        
        // Calculate average export time
        let total_time = stats.avg_export_time_ms * (stats.total_batches - 1) as f64 + export_time_ms as f64;
        stats.avg_export_time_ms = total_time / stats.total_batches as f64;
    }

    /// Get the configuration
    pub fn config(&self) -> &S3ParquetConfig {
        &self.config
    }

    /// Check if the exporter is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Perform health check
    pub async fn health_check(&self) -> S3ParquetResult<bool> {
        self.writer.health_check().await
    }
}

impl Default for ExporterStats {
    fn default() -> Self {
        Self {
            total_batches: 0,
            total_records: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            avg_export_time_ms: 0.0,
            error_count: 0,
            last_export_time: None,
        }
    }
}

#[async_trait]
impl LakehouseExporter for S3ParquetExporter {
    async fn export_batch(&self, batch: TelemetryBatch) -> BridgeResult<ExportResult> {
        let start_time = Instant::now();
        debug!("Exporting telemetry batch to S3/Parquet: {} records", batch.records.len());
        
        match self.export_batch_internal(batch).await {
            Ok(()) => {
                let duration_ms = start_time.elapsed().as_millis() as u64;
                self.update_stats(duration_ms, batch.records.len()).await;
                
                info!("Successfully exported {} records to S3/Parquet in {}ms", batch.records.len(), duration_ms);
                
                Ok(ExportResult {
                    timestamp: chrono::Utc::now(),
                    status: bridge_core::types::ExportStatus::Success,
                    records_failed: 0,
                    duration_ms,
                    metadata: std::collections::HashMap::new(),
                })
            }
            Err(e) => {
                let mut stats = self.stats.write().await;
                stats.error_count += 1;
                drop(stats);
                
                error!("Failed to export batch to S3/Parquet: {:?}", e);
                
                Ok(ExportResult {
                    timestamp: chrono::Utc::now(),
                    status: bridge_core::types::ExportStatus::Failed,
                    records_failed: batch.records.len(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    metadata: std::collections::HashMap::new(),
                })
            }
        }
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ExporterStats> {
        let stats = self.stats.read().await;
        
        Ok(bridge_core::traits::ExporterStats {
            total_batches: stats.total_batches,
            total_records: stats.total_records,
            batches_per_minute: stats.batches_per_minute,
            records_per_minute: stats.records_per_minute,
            avg_export_time_ms: stats.avg_export_time_ms,
            error_count: stats.error_count,
            last_export_time: stats.last_export_time,
        })
    }

    async fn close(&self) -> BridgeResult<()> {
        info!("Closing S3/Parquet exporter");
        
        // Close the writer
        self.writer.close().await
            .map_err(|e| bridge_core::error::BridgeError::lakehouse_with_source("Failed to close writer", e))?;
        
        info!("S3/Parquet exporter closed successfully");
        Ok(())
    }
}

impl S3ParquetExporter {
    /// Export batch internally
    async fn export_batch_internal(&self, batch: TelemetryBatch) -> S3ParquetResult<()> {
        // Use the writer to write the batch
        self.writer.write_batch(batch).await
            .map_err(|e| S3ParquetError::write(format!("Failed to write batch: {}", e)))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::S3ParquetConfig;

    #[tokio::test]
    async fn test_s3_parquet_exporter_creation() {
        let config = S3ParquetConfig {
            bucket_name: "test-bucket".to_string(),
            prefix: "telemetry".to_string(),
            region: "us-east-1".to_string(),
            ..Default::default()
        };

        let exporter = S3ParquetExporter::new(config).await.unwrap();
        assert_eq!(exporter.config().bucket(), "test-bucket");
        assert!(exporter.is_initialized());
    }

    #[tokio::test]
    async fn test_s3_parquet_exporter_health_check() {
        let config = S3ParquetConfig {
            bucket_name: "test-bucket".to_string(),
            prefix: "telemetry".to_string(),
            region: "us-east-1".to_string(),
            ..Default::default()
        };

        let exporter = S3ParquetExporter::new(config).await.unwrap();
        assert!(exporter.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_s3_parquet_exporter_export_batch() {
        let config = S3ParquetConfig {
            bucket_name: "test-bucket".to_string(),
            prefix: "telemetry".to_string(),
            region: "us-east-1".to_string(),
            ..Default::default()
        };

        let exporter = S3ParquetExporter::new(config).await.unwrap();

        // Create a test batch
        let batch = TelemetryBatch {
            metrics: Some(bridge_core::types::MetricsBatch {
                records: vec![
                    bridge_core::types::MetricRecord {
                        name: "test_metric".to_string(),
                        value: 42.0,
                        timestamp: chrono::Utc::now(),
                        labels: vec![("service".to_string(), "test".to_string())],
                    }
                ],
            }),
            traces: None,
            logs: None,
        };

        let result = exporter.export_batch(batch).await;
        assert!(result.is_ok());

        let export_result = result.unwrap();
        assert_eq!(export_result.status, bridge_core::types::ExportStatus::Success);
        assert_eq!(export_result.records_failed, 0);
    }
}
