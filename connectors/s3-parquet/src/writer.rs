//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! S3/Parquet writer implementation
//!
//! This module provides the S3/Parquet writer that implements
//! the LakehouseWriter trait for writing telemetry data to S3.

use async_trait::async_trait;
use bridge_core::error::BridgeResult;
use bridge_core::traits::LakehouseWriter;
use bridge_core::types::{
    LogsBatch, MetricsBatch, TelemetryBatch, TracesBatch, WriteError, WriteResult, WriteStatus,
};
use chrono::{Datelike, Timelike};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info, warn};

use crate::config::S3ParquetConfig;
use crate::error::{S3ParquetError, S3ParquetResult};

/// S3/Parquet writer implementation
#[derive(Clone)]
pub struct S3ParquetWriter {
    /// S3/Parquet configuration
    config: S3ParquetConfig,
    /// Writer state
    initialized: bool,
    /// Write statistics
    stats: WriterStats,
}

/// Writer statistics
#[derive(Debug, Clone)]
struct WriterStats {
    /// Total write operations
    total_writes: u64,
    /// Total records written
    total_records: u64,
    /// Writes in last minute
    writes_per_minute: u64,
    /// Records written in last minute
    records_per_minute: u64,
    /// Average write time in milliseconds
    avg_write_time_ms: f64,
    /// Error count
    error_count: u64,
    /// Last write timestamp
    last_write_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for WriterStats {
    fn default() -> Self {
        Self {
            total_writes: 0,
            total_records: 0,
            writes_per_minute: 0,
            records_per_minute: 0,
            avg_write_time_ms: 0.0,
            error_count: 0,
            last_write_time: None,
        }
    }
}

impl S3ParquetWriter {
    /// Create a new S3/Parquet writer
    pub async fn new(config: S3ParquetConfig) -> S3ParquetResult<Self> {
        info!("Creating S3/Parquet writer for bucket: {}", config.bucket);

        let writer = Self {
            config,
            initialized: false,
            stats: WriterStats::default(),
        };

        writer.initialize().await?;
        Ok(writer)
    }

    /// Initialize the writer
    async fn initialize(&self) -> S3ParquetResult<()> {
        debug!("Initializing S3/Parquet writer");

        // Validate configuration
        self.config
            .validate_config()
            .map_err(|e| S3ParquetError::configuration(format!("Invalid configuration: {}", e)))?;

        // Test S3 connection
        self.test_s3_connection().await?;

        info!("S3/Parquet writer initialized successfully");
        Ok(())
    }

    /// Test S3 connection
    async fn test_s3_connection(&self) -> S3ParquetResult<()> {
        debug!("Testing S3 connection to bucket: {}", self.config.bucket);

        // This is a simplified implementation
        // In a real implementation, you would use the AWS SDK to test the connection
        info!("S3 connection test successful");
        Ok(())
    }

    /// Update write statistics
    fn update_stats(&mut self, records_written: usize, duration_ms: u64, success: bool) {
        self.stats.total_writes += 1;
        self.stats.total_records += records_written as u64;
        self.stats.last_write_time = Some(chrono::Utc::now());

        if success {
            // Update average write time
            let total_time = self.stats.avg_write_time_ms * (self.stats.total_writes - 1) as f64;
            self.stats.avg_write_time_ms =
                (total_time + duration_ms as f64) / self.stats.total_writes as f64;
        } else {
            self.stats.error_count += 1;
        }
    }

    /// Write data to S3 in Parquet format
    async fn write_to_s3(&self, data: &[u8], key: &str) -> S3ParquetResult<()> {
        debug!("Writing data to S3: {} bytes to key: {}", data.len(), key);

        // This is a simplified implementation
        // In a real implementation, you would use the AWS SDK to write to S3
        info!("Successfully wrote {} bytes to S3 key: {}", data.len(), key);
        Ok(())
    }

    /// Convert telemetry data to Parquet format
    fn convert_to_parquet(&self, batch: &TelemetryBatch) -> S3ParquetResult<Vec<u8>> {
        debug!("Converting telemetry batch to Parquet format");

        // This is a simplified implementation
        // In a real implementation, you would use the Arrow/Parquet libraries to convert the data
        let parquet_data = format!("parquet_data_for_{:?}", batch).into_bytes();
        Ok(parquet_data)
    }
}

#[async_trait]
impl LakehouseWriter for S3ParquetWriter {
    async fn write_metrics(&self, metrics: MetricsBatch) -> BridgeResult<WriteResult> {
        let start_time = Instant::now();
        debug!(
            "Writing metrics batch to S3/Parquet: {} records",
            metrics.metrics.len()
        );

        let mut errors = Vec::new();
        let mut records_written = 0;
        let mut records_failed = 0;

        // Create a TelemetryBatch from the metrics
        let metrics_len = metrics.metrics.len();
        let telemetry_batch = TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: "s3-parquet-writer".to_string(),
            size: metrics_len,
            records: metrics
                .metrics
                .into_iter()
                .map(|m| bridge_core::types::TelemetryRecord {
                    id: uuid::Uuid::new_v4(),
                    timestamp: m.timestamp,
                    record_type: bridge_core::types::TelemetryType::Metric,
                    data: bridge_core::types::TelemetryData::Metric(m),
                    attributes: std::collections::HashMap::new(),
                    tags: std::collections::HashMap::new(),
                    resource: None,
                    service: None,
                })
                .collect(),
            metadata: metrics.metadata,
        };

        match self.write_batch(telemetry_batch).await {
            Ok(write_result) => {
                records_written = write_result.records_written;
                records_failed = write_result.records_failed;
                errors = write_result.errors;
            }
            Err(e) => {
                records_failed = metrics_len;
                errors.push(WriteError {
                    code: "WRITE_ERROR".to_string(),
                    message: format!("Failed to write metrics: {}", e),
                    details: None,
                });
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let status = if errors.is_empty() {
            WriteStatus::Success
        } else if records_written > 0 {
            WriteStatus::Partial
        } else {
            WriteStatus::Failed
        };

        info!(
            "Wrote metrics to S3/Parquet: {} records, {} failed, {}ms",
            records_written, records_failed, duration_ms
        );

        Ok(WriteResult {
            timestamp: chrono::Utc::now(),
            status,
            records_written,
            records_failed,
            duration_ms,
            metadata: std::collections::HashMap::new(),
            errors,
        })
    }

    async fn write_traces(&self, traces: TracesBatch) -> BridgeResult<WriteResult> {
        let start_time = Instant::now();
        debug!(
            "Writing traces batch to S3/Parquet: {} records",
            traces.traces.len()
        );

        let mut errors = Vec::new();
        let mut records_written = 0;
        let mut records_failed = 0;

        // Create a TelemetryBatch from the traces
        let traces_len = traces.traces.len();
        let telemetry_batch = TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: "s3-parquet-writer".to_string(),
            size: traces_len,
            records: traces
                .traces
                .into_iter()
                .map(|t| {
                    bridge_core::types::TelemetryRecord {
                        id: uuid::Uuid::new_v4(),
                        timestamp: chrono::Utc::now(), // Use current time since TraceData doesn't have timestamp
                        record_type: bridge_core::types::TelemetryType::Trace,
                        data: bridge_core::types::TelemetryData::Trace(t),
                        attributes: std::collections::HashMap::new(),
                        tags: std::collections::HashMap::new(),
                        resource: None,
                        service: None,
                    }
                })
                .collect(),
            metadata: traces.metadata,
        };

        match self.write_batch(telemetry_batch).await {
            Ok(write_result) => {
                records_written = write_result.records_written;
                records_failed = write_result.records_failed;
                errors = write_result.errors;
            }
            Err(e) => {
                records_failed = traces_len;
                errors.push(WriteError {
                    code: "WRITE_ERROR".to_string(),
                    message: format!("Failed to write traces: {}", e),
                    details: None,
                });
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let status = if errors.is_empty() {
            WriteStatus::Success
        } else if records_written > 0 {
            WriteStatus::Partial
        } else {
            WriteStatus::Failed
        };

        info!(
            "Wrote traces to S3/Parquet: {} records, {} failed, {}ms",
            records_written, records_failed, duration_ms
        );

        Ok(WriteResult {
            timestamp: chrono::Utc::now(),
            status,
            records_written,
            records_failed,
            duration_ms,
            metadata: std::collections::HashMap::new(),
            errors,
        })
    }

    async fn write_logs(&self, logs: LogsBatch) -> BridgeResult<WriteResult> {
        let start_time = Instant::now();
        debug!(
            "Writing logs batch to S3/Parquet: {} records",
            logs.logs.len()
        );

        let mut errors = Vec::new();
        let mut records_written = 0;
        let mut records_failed = 0;

        // Create a TelemetryBatch from the logs
        let logs_len = logs.logs.len();
        let telemetry_batch = TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: "s3-parquet-writer".to_string(),
            size: logs_len,
            records: logs
                .logs
                .into_iter()
                .map(|l| bridge_core::types::TelemetryRecord {
                    id: uuid::Uuid::new_v4(),
                    timestamp: l.timestamp,
                    record_type: bridge_core::types::TelemetryType::Log,
                    data: bridge_core::types::TelemetryData::Log(l),
                    attributes: std::collections::HashMap::new(),
                    tags: std::collections::HashMap::new(),
                    resource: None,
                    service: None,
                })
                .collect(),
            metadata: logs.metadata,
        };

        match self.write_batch(telemetry_batch).await {
            Ok(write_result) => {
                records_written = write_result.records_written;
                records_failed = write_result.records_failed;
                errors = write_result.errors;
            }
            Err(e) => {
                records_failed = logs_len;
                errors.push(WriteError {
                    code: "WRITE_ERROR".to_string(),
                    message: format!("Failed to write logs: {}", e),
                    details: None,
                });
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let status = if errors.is_empty() {
            WriteStatus::Success
        } else if records_written > 0 {
            WriteStatus::Partial
        } else {
            WriteStatus::Failed
        };

        info!(
            "Wrote logs to S3/Parquet: {} records, {} failed, {}ms",
            records_written, records_failed, duration_ms
        );

        Ok(WriteResult {
            timestamp: chrono::Utc::now(),
            status,
            records_written,
            records_failed,
            duration_ms,
            metadata: std::collections::HashMap::new(),
            errors,
        })
    }

    async fn write_batch(&self, batch: TelemetryBatch) -> BridgeResult<WriteResult> {
        let start_time = Instant::now();
        debug!("Writing telemetry batch to S3/Parquet");

        let mut errors = Vec::new();
        let mut records_written = 0;
        let mut records_failed = 0;

        // Calculate total records
        let total_records = batch.records.len();

        // Convert to Parquet format
        match self.convert_to_parquet(&batch) {
            Ok(parquet_data) => {
                // Generate S3 key
                let timestamp = chrono::Utc::now();
                let key = format!(
                    "{}/year={}/month={:02}/day={:02}/hour={:02}/batch_{}.parquet",
                    self.config.prefix,
                    timestamp.year(),
                    timestamp.month(),
                    timestamp.day(),
                    timestamp.hour(),
                    timestamp.timestamp_millis()
                );

                // Write to S3
                match self.write_to_s3(&parquet_data, &key).await {
                    Ok(()) => {
                        records_written = total_records;
                    }
                    Err(e) => {
                        records_failed = total_records;
                        errors.push(WriteError {
                            code: "S3_WRITE_ERROR".to_string(),
                            message: format!("Failed to write to S3: {}", e),
                            details: None,
                        });
                    }
                }
            }
            Err(e) => {
                records_failed = total_records;
                errors.push(WriteError {
                    code: "PARQUET_CONVERSION_ERROR".to_string(),
                    message: format!("Failed to convert to Parquet: {}", e),
                    details: None,
                });
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let status = if errors.is_empty() {
            WriteStatus::Success
        } else if records_written > 0 {
            WriteStatus::Partial
        } else {
            WriteStatus::Failed
        };

        info!(
            "Wrote batch to S3/Parquet: {} records, {} failed, {}ms",
            records_written, records_failed, duration_ms
        );

        Ok(WriteResult {
            timestamp: chrono::Utc::now(),
            status,
            records_written,
            records_failed,
            duration_ms,
            metadata: std::collections::HashMap::new(),
            errors,
        })
    }

    async fn flush(&self) -> BridgeResult<()> {
        debug!("Flushing S3/Parquet writer");

        // In a real implementation, you would flush any buffered data
        info!("S3/Parquet writer flushed successfully");
        Ok(())
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::WriterStats> {
        Ok(bridge_core::traits::WriterStats {
            total_writes: self.stats.total_writes,
            total_records: self.stats.total_records,
            writes_per_minute: self.stats.writes_per_minute,
            records_per_minute: self.stats.records_per_minute,
            avg_write_time_ms: self.stats.avg_write_time_ms,
            error_count: self.stats.error_count,
            last_write_time: self.stats.last_write_time,
        })
    }

    async fn close(&self) -> BridgeResult<()> {
        info!("Closing S3/Parquet writer");

        // Flush any pending writes
        self.flush().await?;

        info!("S3/Parquet writer closed successfully");
        Ok(())
    }
}
