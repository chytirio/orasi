//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! S3/Parquet exporter implementation
//!
//! This module provides the S3/Parquet exporter that implements
//! the LakehouseExporter trait for exporting telemetry data to S3.

use async_trait::async_trait;
use bridge_core::error::BridgeResult;
use bridge_core::traits::LakehouseExporter;
use bridge_core::traits::LakehouseWriter;
use bridge_core::types::{
    ExportError, ExportResult, ExportStatus, ProcessedBatch, TelemetryBatch, TelemetryData,
    TelemetryRecord, TelemetryType,
};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info, warn};

use crate::config::S3ParquetConfig;
use crate::error::{S3ParquetError, S3ParquetResult};
use crate::writer::S3ParquetWriter;

/// S3/Parquet exporter implementation
pub struct S3ParquetExporter {
    /// S3/Parquet configuration
    config: S3ParquetConfig,
    /// Writer instance
    writer: S3ParquetWriter,
    /// Export statistics
    stats: ExportStats,
}

/// Export statistics
#[derive(Debug, Clone)]
struct ExportStats {
    /// Total batches exported
    total_batches: u64,
    /// Total records exported
    total_records: u64,
    /// Batches exported in last minute
    batches_per_minute: u64,
    /// Records exported in last minute
    records_per_minute: u64,
    /// Average export time in milliseconds
    avg_export_time_ms: f64,
    /// Error count
    error_count: u64,
    /// Last export timestamp
    last_export_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for ExportStats {
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

impl S3ParquetExporter {
    /// Create a new S3/Parquet exporter
    pub async fn new(config: S3ParquetConfig) -> S3ParquetResult<Self> {
        info!("Creating S3/Parquet exporter for bucket: {}", config.bucket);

        // Initialize writer
        let writer = S3ParquetWriter::new(config.clone()).await?;

        Ok(Self {
            config,
            writer,
            stats: ExportStats::default(),
        })
    }

    /// Get the configuration
    pub fn config(&self) -> &S3ParquetConfig {
        &self.config
    }

    /// Update export statistics
    fn update_stats(&mut self, records_exported: usize, duration_ms: u64, success: bool) {
        self.stats.total_batches += 1;
        self.stats.total_records += records_exported as u64;
        self.stats.last_export_time = Some(chrono::Utc::now());

        if success {
            // Update average export time
            let total_time = self.stats.avg_export_time_ms * (self.stats.total_batches - 1) as f64;
            self.stats.avg_export_time_ms =
                (total_time + duration_ms as f64) / self.stats.total_batches as f64;
        } else {
            self.stats.error_count += 1;
        }
    }

    /// Convert ProcessedBatch to TelemetryBatch
    fn convert_to_telemetry_batch(&self, batch: ProcessedBatch) -> TelemetryBatch {
        TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: batch.timestamp,
            source: "s3-parquet-exporter".to_string(),
            size: batch.records.len(),
            records: batch
                .records
                .into_iter()
                .filter_map(|record| {
                    // Only include records with successful processing and transformed data
                    if record.status == bridge_core::types::ProcessingStatus::Success {
                        record.transformed_data.map(|data| {
                            TelemetryRecord {
                                id: record.original_id,
                                timestamp: chrono::Utc::now(), // Use current time for export
                                record_type: match &data {
                                    TelemetryData::Metric(_) => TelemetryType::Metric,
                                    TelemetryData::Trace(_) => TelemetryType::Trace,
                                    TelemetryData::Log(_) => TelemetryType::Log,
                                    TelemetryData::Event(_) => TelemetryType::Event,
                                },
                                data,
                                attributes: std::collections::HashMap::new(),
                                tags: std::collections::HashMap::new(),
                                resource: None,
                                service: None,
                            }
                        })
                    } else {
                        None
                    }
                })
                .collect(),
            metadata: batch.metadata,
        }
    }
}

#[async_trait]
impl LakehouseExporter for S3ParquetExporter {
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        let start_time = Instant::now();
        debug!(
            "Exporting processed batch to S3/Parquet: {} records",
            batch.records.len()
        );

        let mut errors = Vec::new();
        let mut records_exported = 0;
        let mut records_failed = 0;

        // Convert ProcessedBatch to TelemetryBatch and write
        let batch_len = batch.records.len();
        let telemetry_batch = self.convert_to_telemetry_batch(batch);
        match self.writer.write_batch(telemetry_batch).await {
            Ok(write_result) => {
                records_exported = write_result.records_written;
                records_failed = write_result.records_failed;

                // Convert write errors to export errors
                for write_error in write_result.errors {
                    errors.push(ExportError {
                        code: write_error.code,
                        message: write_error.message,
                        details: write_error.details,
                    });
                }
            }
            Err(e) => {
                records_failed = batch_len;
                errors.push(ExportError {
                    code: "EXPORT_ERROR".to_string(),
                    message: format!("Failed to export batch: {}", e),
                    details: None,
                });
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let status = if errors.is_empty() {
            ExportStatus::Success
        } else if records_exported > 0 {
            ExportStatus::Partial
        } else {
            ExportStatus::Failed
        };

        info!(
            "Exported batch to S3/Parquet: {} records, {} failed, {}ms",
            records_exported, records_failed, duration_ms
        );

        Ok(ExportResult {
            timestamp: chrono::Utc::now(),
            status,
            records_exported,
            records_failed,
            duration_ms,
            metadata: std::collections::HashMap::new(),
            errors,
        })
    }

    fn name(&self) -> &str {
        "s3-parquet-exporter"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        // Check if writer is healthy
        match self.writer.get_stats().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ExporterStats> {
        Ok(bridge_core::traits::ExporterStats {
            total_batches: self.stats.total_batches,
            total_records: self.stats.total_records,
            batches_per_minute: self.stats.batches_per_minute,
            records_per_minute: self.stats.records_per_minute,
            avg_export_time_ms: self.stats.avg_export_time_ms,
            error_count: self.stats.error_count,
            last_export_time: self.stats.last_export_time,
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down S3/Parquet exporter");

        // Close the writer
        self.writer.close().await.map_err(|e| {
            bridge_core::error::BridgeError::lakehouse_with_source("Failed to close writer", e)
        })?;

        info!("S3/Parquet exporter shutdown complete");
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
            bucket: "test-bucket".to_string(),
            prefix: "telemetry".to_string(),
            region: "us-east-1".to_string(),
            ..Default::default()
        };

        let exporter = S3ParquetExporter::new(config).await.unwrap();
        assert_eq!(exporter.config().bucket, "test-bucket");
    }

    #[tokio::test]
    async fn test_s3_parquet_exporter_health_check() {
        let config = S3ParquetConfig {
            bucket: "test-bucket".to_string(),
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
            bucket: "test-bucket".to_string(),
            prefix: "telemetry".to_string(),
            region: "us-east-1".to_string(),
            ..Default::default()
        };

        let exporter = S3ParquetExporter::new(config).await.unwrap();

        // Create a test batch
        let batch = ProcessedBatch {
            original_batch_id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            status: bridge_core::types::ProcessingStatus::Success,
            records: vec![bridge_core::types::ProcessedRecord {
                original_id: uuid::Uuid::new_v4(),
                status: bridge_core::types::ProcessingStatus::Success,
                transformed_data: Some(bridge_core::types::TelemetryData::Metric(
                    bridge_core::types::MetricData {
                        name: "test_metric".to_string(),
                        description: Some("Test metric".to_string()),
                        unit: Some("count".to_string()),
                        metric_type: bridge_core::types::MetricType::Counter,
                        value: bridge_core::types::MetricValue::Counter(42.0),
                        labels: std::collections::HashMap::from([(
                            "service".to_string(),
                            "test".to_string(),
                        )]),
                        timestamp: chrono::Utc::now(),
                    },
                )),
                metadata: std::collections::HashMap::new(),
                errors: Vec::new(),
            }],
            metadata: std::collections::HashMap::new(),
            errors: Vec::new(),
        };

        let result = exporter.export(batch).await;
        assert!(result.is_ok());

        let export_result = result.unwrap();
        assert_eq!(
            export_result.status,
            bridge_core::types::ExportStatus::Success
        );
        assert_eq!(export_result.records_failed, 0);
    }
}
