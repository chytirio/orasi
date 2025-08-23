//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Parquet exporter for the OpenTelemetry Data Lake Bridge
//!
//! This module provides a Parquet exporter that can export telemetry data
//! to Parquet format files.

use crate::error::BridgeResult;
use crate::health::checker::HealthCheckCallback;
use crate::health::types::HealthCheckResult;
use crate::traits::exporter::LakehouseExporter;
use crate::types::{ExportResult, ProcessedBatch, TelemetryData, TelemetryRecord, TelemetryType};
use arrow::array::{
    ArrayRef, BooleanArray, Float64Array, Int64Array, StringArray, TimestampNanosecondArray,
    UInt64Array,
};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Parquet exporter configuration
#[derive(Debug, Clone)]
pub struct ParquetExporterConfig {
    /// Output directory for Parquet files
    pub output_directory: String,

    /// File prefix for generated files
    pub file_prefix: String,

    /// Maximum records per file
    pub max_records_per_file: usize,

    /// Compression type
    pub compression: ParquetCompression,

    /// Whether to partition data
    pub enable_partitioning: bool,

    /// Partition columns
    pub partition_columns: Vec<String>,

    /// Whether to create output directory if it doesn't exist
    pub create_directory: bool,
}

/// Parquet compression types
#[derive(Debug, Clone)]
pub enum ParquetCompression {
    None,
    Snappy,
    Gzip,
    Lz4,
    Zstd,
}

impl Default for ParquetExporterConfig {
    fn default() -> Self {
        Self {
            output_directory: "./data".to_string(),
            file_prefix: "telemetry".to_string(),
            max_records_per_file: 10000,
            compression: ParquetCompression::Snappy,
            enable_partitioning: false,
            partition_columns: Vec::new(),
            create_directory: true,
        }
    }
}

/// Parquet exporter statistics
#[derive(Debug, Clone)]
pub struct ParquetExporterStats {
    /// Total batches exported
    pub total_batches: u64,

    /// Total records exported
    pub total_records: u64,

    /// Total bytes exported
    pub total_bytes: u64,

    /// Batches exported in last minute
    pub batches_per_minute: u64,

    /// Records exported in last minute
    pub records_per_minute: u64,

    /// Average export time in milliseconds
    pub avg_export_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last export timestamp
    pub last_export_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Files created
    pub files_created: u64,
}

impl Default for ParquetExporterStats {
    fn default() -> Self {
        Self {
            total_batches: 0,
            total_records: 0,
            total_bytes: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            avg_export_time_ms: 0.0,
            error_count: 0,
            last_export_time: None,
            files_created: 0,
        }
    }
}

/// Parquet exporter for exporting telemetry data
#[derive(Debug)]
pub struct ParquetExporter {
    /// Exporter configuration
    pub config: ParquetExporterConfig,

    /// Exporter statistics
    pub stats: Arc<RwLock<ParquetExporterStats>>,

    /// Running state
    pub running: Arc<RwLock<bool>>,

    /// Current file record count
    pub current_file_records: Arc<RwLock<usize>>,

    /// Current file path
    pub current_file_path: Arc<RwLock<Option<String>>>,
}

impl ParquetExporter {
    /// Create a new Parquet exporter
    pub fn new(config: ParquetExporterConfig) -> Self {
        info!(
            "Creating Parquet exporter with output directory: {}",
            config.output_directory
        );

        Self {
            config,
            stats: Arc::new(RwLock::new(ParquetExporterStats::default())),
            running: Arc::new(RwLock::new(false)),
            current_file_records: Arc::new(RwLock::new(0)),
            current_file_path: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the Parquet exporter
    pub async fn start(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(crate::error::BridgeError::internal(
                "Parquet exporter is already running",
            ));
        }

        *running = true;
        info!("Starting Parquet exporter");

        // Create output directory if needed
        if self.config.create_directory {
            self.ensure_output_directory().await?;
        }

        Ok(())
    }

    /// Stop the Parquet exporter
    pub async fn stop(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Parquet exporter is not running",
            ));
        }

        *running = false;
        info!("Stopping Parquet exporter");

        Ok(())
    }

    /// Ensure output directory exists
    async fn ensure_output_directory(&self) -> BridgeResult<()> {
        let path = Path::new(&self.config.output_directory);
        if !path.exists() {
            tokio::fs::create_dir_all(path).await.map_err(|e| {
                crate::error::BridgeError::configuration(format!(
                    "Failed to create output directory: {}",
                    e
                ))
            })?;
            info!("Created output directory: {}", self.config.output_directory);
        }
        Ok(())
    }

    /// Generate file path for current batch
    fn generate_file_path(&self) -> String {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let uuid_string = Uuid::new_v4().to_string();
        let uuid = uuid_string.split('-').next().unwrap();
        format!(
            "{}/{}_{}_{}.parquet",
            self.config.output_directory, self.config.file_prefix, timestamp, uuid
        )
    }

    /// Convert compression type to Parquet compression
    fn get_parquet_compression(&self) -> parquet::basic::Compression {
        match self.config.compression {
            ParquetCompression::None => parquet::basic::Compression::UNCOMPRESSED,
            ParquetCompression::Snappy => parquet::basic::Compression::SNAPPY,
            ParquetCompression::Gzip => {
                parquet::basic::Compression::GZIP(parquet::basic::GzipLevel::default())
            }
            ParquetCompression::Lz4 => parquet::basic::Compression::LZ4,
            ParquetCompression::Zstd => {
                parquet::basic::Compression::ZSTD(parquet::basic::ZstdLevel::default())
            }
        }
    }

    /// Convert processed records to Arrow arrays
    fn records_to_arrow_arrays(
        &self,
        records: &[crate::types::ProcessedRecord],
    ) -> BridgeResult<Vec<ArrayRef>> {
        let mut ids = Vec::with_capacity(records.len());
        let mut timestamps = Vec::with_capacity(records.len());
        let mut types = Vec::with_capacity(records.len());
        let mut sources = Vec::with_capacity(records.len());
        let mut trace_ids = Vec::with_capacity(records.len());
        let mut span_ids = Vec::with_capacity(records.len());
        let mut names = Vec::with_capacity(records.len());
        let mut values = Vec::with_capacity(records.len());
        let mut levels = Vec::with_capacity(records.len());
        let mut messages = Vec::with_capacity(records.len());
        let mut attributes = Vec::with_capacity(records.len());

        for record in records {
            ids.push(record.original_id.to_string());
            timestamps.push(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
            types.push(format!("{:?}", record.status));
            sources.push("processed".to_string());

            // Extract telemetry-specific data from transformed data
            if let Some(ref data) = record.transformed_data {
                match data {
                    TelemetryData::Trace(trace) => {
                        trace_ids.push(Some(trace.trace_id.clone()));
                        span_ids.push(Some(trace.span_id.clone()));
                        names.push(Some(trace.name.clone()));
                        values.push(None);
                        levels.push(None);
                        messages.push(None);
                    }
                    TelemetryData::Metric(metric) => {
                        trace_ids.push(None);
                        span_ids.push(None);
                        names.push(Some(metric.name.clone()));
                        values.push(Some(format!("{:?}", metric.value)));
                        levels.push(None);
                        messages.push(None);
                    }
                    TelemetryData::Log(log) => {
                        trace_ids.push(None);
                        span_ids.push(None);
                        names.push(None);
                        values.push(None);
                        levels.push(Some(format!("{:?}", log.level)));
                        messages.push(Some(log.message.clone()));
                    }
                    TelemetryData::Event(event) => {
                        trace_ids.push(None);
                        span_ids.push(None);
                        names.push(Some(event.name.clone()));
                        values.push(None);
                        levels.push(None);
                        messages.push(None);
                    }
                }
            } else {
                // No transformed data available
                trace_ids.push(None);
                span_ids.push(None);
                names.push(None);
                values.push(None);
                levels.push(None);
                messages.push(None);
            }

            // Convert metadata to JSON string
            let attrs_json =
                serde_json::to_string(&record.metadata).unwrap_or_else(|_| "{}".to_string());
            attributes.push(attrs_json);
        }

        Ok(vec![
            Arc::new(StringArray::from(ids)),
            Arc::new(TimestampNanosecondArray::from(timestamps)),
            Arc::new(StringArray::from(types)),
            Arc::new(StringArray::from(sources)),
            Arc::new(StringArray::from(trace_ids)),
            Arc::new(StringArray::from(span_ids)),
            Arc::new(StringArray::from(names)),
            Arc::new(StringArray::from(values)),
            Arc::new(StringArray::from(levels)),
            Arc::new(StringArray::from(messages)),
            Arc::new(StringArray::from(attributes)),
        ])
    }

    /// Create Arrow schema for telemetry data
    fn create_schema() -> Schema {
        Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Nanosecond, None),
                false,
            ),
            Field::new("type", DataType::Utf8, false),
            Field::new("source", DataType::Utf8, false),
            Field::new("trace_id", DataType::Utf8, true),
            Field::new("span_id", DataType::Utf8, true),
            Field::new("name", DataType::Utf8, true),
            Field::new("value", DataType::Utf8, true),
            Field::new("level", DataType::Utf8, true),
            Field::new("message", DataType::Utf8, true),
            Field::new("attributes", DataType::Utf8, false),
        ])
    }

    /// Write records to Parquet file
    async fn write_to_parquet(
        &self,
        records: &[crate::types::ProcessedRecord],
        file_path: &str,
    ) -> BridgeResult<usize> {
        let start_time = std::time::Instant::now();

        // Convert records to Arrow arrays
        let arrays = self.records_to_arrow_arrays(records)?;
        let schema = Self::create_schema();

        // Create record batch
        let batch = RecordBatch::try_new(Arc::new(schema), arrays).map_err(|e| {
            crate::error::BridgeError::internal(format!("Failed to create record batch: {}", e))
        })?;

        // Create file
        let file = File::create(file_path).map_err(|e| {
            crate::error::BridgeError::internal(format!(
                "Failed to create file {}: {}",
                file_path, e
            ))
        })?;

        // Configure writer properties
        let props = WriterProperties::builder()
            .set_compression(self.get_parquet_compression())
            .build();

        // Write to Parquet
        let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props)).map_err(|e| {
            crate::error::BridgeError::internal(format!("Failed to create Parquet writer: {}", e))
        })?;

        writer.write(&batch).map_err(|e| {
            crate::error::BridgeError::internal(format!("Failed to write batch: {}", e))
        })?;

        writer.close().map_err(|e| {
            crate::error::BridgeError::internal(format!("Failed to close writer: {}", e))
        })?;

        let duration = start_time.elapsed();
        let bytes_written = std::fs::metadata(file_path)
            .map(|m| m.len() as usize)
            .unwrap_or(0);

        info!(
            "Wrote {} records to {} in {:?} ({} bytes)",
            records.len(),
            file_path,
            duration,
            bytes_written
        );

        Ok(bytes_written)
    }
}

#[async_trait]
impl LakehouseExporter for ParquetExporter {
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        let start_time = std::time::Instant::now();

        let running = self.running.read().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Parquet exporter is not running",
            ));
        }

        let record_count = batch.records.len();
        if record_count == 0 {
            return Ok(ExportResult {
                timestamp: chrono::Utc::now(),
                status: crate::types::ExportStatus::Success,
                records_exported: 0,
                records_failed: 0,
                duration_ms: 0,
                metadata: HashMap::new(),
                errors: vec![],
            });
        }

        // Check if we need to create a new file
        let mut current_records = self.current_file_records.write().await;
        let mut current_file_path = self.current_file_path.write().await;

        let file_path = if *current_records >= self.config.max_records_per_file
            || current_file_path.is_none()
        {
            // Create new file
            let new_path = self.generate_file_path();
            *current_records = 0;
            *current_file_path = Some(new_path.clone());
            new_path
        } else {
            current_file_path.as_ref().unwrap().clone()
        };

        // Write records to Parquet file
        let bytes_written = match self.write_to_parquet(&batch.records, &file_path).await {
            Ok(bytes) => bytes,
            Err(e) => {
                // Update statistics
                {
                    let mut stats = self.stats.write().await;
                    stats.error_count += 1;
                    stats.last_export_time = Some(chrono::Utc::now());
                }
                return Err(e);
            }
        };

        *current_records += record_count;

        let duration = start_time.elapsed();
        let duration_ms = duration.as_millis() as u64;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_batches += 1;
            stats.total_records += record_count as u64;
            stats.total_bytes += bytes_written as u64;
            stats.last_export_time = Some(chrono::Utc::now());
            stats.files_created += if *current_records == record_count {
                1
            } else {
                0
            };

            // Update average export time
            if stats.total_batches > 0 {
                stats.avg_export_time_ms = (stats.avg_export_time_ms
                    * (stats.total_batches - 1) as f64
                    + duration_ms as f64)
                    / stats.total_batches as f64;
            }
        }

        Ok(ExportResult {
            timestamp: chrono::Utc::now(),
            status: crate::types::ExportStatus::Success,
            records_exported: record_count,
            records_failed: 0,
            duration_ms,
            metadata: HashMap::from([
                (
                    "destination".to_string(),
                    format!("parquet://{}", file_path),
                ),
                ("bytes_exported".to_string(), bytes_written.to_string()),
                ("file_path".to_string(), file_path),
            ]),
            errors: vec![],
        })
    }

    fn name(&self) -> &str {
        "parquet_exporter"
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
        Ok(crate::traits::exporter::ExporterStats {
            total_batches: stats.total_batches,
            total_records: stats.total_records,
            batches_per_minute: stats.batches_per_minute,
            records_per_minute: stats.records_per_minute,
            avg_export_time_ms: stats.avg_export_time_ms,
            error_count: stats.error_count,
            last_export_time: stats.last_export_time,
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        self.stop().await
    }
}

#[async_trait]
impl HealthCheckCallback for ParquetExporter {
    async fn check(&self) -> BridgeResult<HealthCheckResult> {
        let running = self.running.read().await;
        let status = if *running {
            crate::health::types::HealthStatus::Healthy
        } else {
            crate::health::types::HealthStatus::Unhealthy
        };

        Ok(HealthCheckResult::new(
            "parquet_exporter".to_string(),
            status,
            if *running {
                "Parquet exporter is running".to_string()
            } else {
                "Parquet exporter is not running".to_string()
            },
        ))
    }

    fn component_name(&self) -> &str {
        "parquet_exporter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TelemetryBatch, TelemetryData, TelemetryRecord, TelemetryType};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_parquet_exporter_creation() {
        let config = ParquetExporterConfig::default();
        let exporter = ParquetExporter::new(config);

        let stats = exporter.get_stats().await.unwrap();
        assert_eq!(stats.total_records, 0);
    }

    #[tokio::test]
    async fn test_parquet_exporter_start_stop() {
        let temp_dir = TempDir::new().unwrap();
        let config = ParquetExporterConfig {
            output_directory: temp_dir.path().to_string_lossy().to_string(),
            create_directory: true,
            ..Default::default()
        };

        let exporter = ParquetExporter::new(config);

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
    async fn test_parquet_exporter_export() {
        let temp_dir = TempDir::new().unwrap();
        let config = ParquetExporterConfig {
            output_directory: temp_dir.path().to_string_lossy().to_string(),
            create_directory: true,
            max_records_per_file: 100,
            ..Default::default()
        };

        let exporter = ParquetExporter::new(config);
        exporter.start().await.unwrap();

        // Create test batch
        let test_record = TelemetryRecord::new(
            TelemetryType::Trace,
            TelemetryData::Trace(crate::types::traces::TraceData {
                trace_id: "test_trace".to_string(),
                span_id: "test_span".to_string(),
                parent_span_id: None,
                name: "test_span".to_string(),
                kind: crate::types::traces::SpanKind::Internal,
                start_time: chrono::Utc::now(),
                end_time: None,
                duration_ns: None,
                status: crate::types::traces::SpanStatus {
                    code: crate::types::traces::StatusCode::Ok,
                    message: None,
                },
                attributes: HashMap::new(),
                events: Vec::new(),
                links: Vec::new(),
            }),
        );

        let processed_record = crate::types::ProcessedRecord {
            original_id: Uuid::new_v4(),
            status: crate::types::ProcessingStatus::Success,
            transformed_data: Some(TelemetryData::Trace(crate::types::traces::TraceData {
                trace_id: "test_trace".to_string(),
                span_id: "test_span".to_string(),
                parent_span_id: None,
                name: "test_span".to_string(),
                kind: crate::types::traces::SpanKind::Internal,
                start_time: chrono::Utc::now(),
                end_time: None,
                duration_ns: None,
                status: crate::types::traces::SpanStatus {
                    code: crate::types::traces::StatusCode::Ok,
                    message: None,
                },
                attributes: HashMap::new(),
                events: Vec::new(),
                links: Vec::new(),
            })),
            metadata: HashMap::new(),
            errors: Vec::new(),
        };

        let batch = ProcessedBatch {
            original_batch_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            status: crate::types::ProcessingStatus::Success,
            records: vec![processed_record],
            metadata: HashMap::new(),
            errors: Vec::new(),
        };

        // Export batch
        let result = exporter.export(batch).await;
        assert!(result.is_ok());

        let export_result = result.unwrap();
        assert_eq!(export_result.records_exported, 1);
        assert_eq!(export_result.records_failed, 0);
        assert_eq!(export_result.status, crate::types::ExportStatus::Success);

        // Check that file was created
        let files: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map_or(false, |ext| ext == "parquet")
            })
            .collect();

        assert_eq!(files.len(), 1);

        exporter.stop().await.unwrap();
    }
}
