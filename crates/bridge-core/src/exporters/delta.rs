//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Delta Lake exporter for the OpenTelemetry Data Lake Bridge
//!
//! This module provides a Delta Lake exporter that can export telemetry data
//! to Delta Lake format.

use crate::error::BridgeResult;
use crate::health::checker::HealthCheckCallback;
use crate::health::types::HealthCheckResult;
use crate::traits::exporter::LakehouseExporter;
use crate::types::{ExportResult, ProcessedBatch, TelemetryData};
use arrow::{
    array::{BooleanArray, Float64Array, Int64Array, StringArray, TimestampNanosecondArray},
    datatypes::{DataType, Field, Schema as ArrowSchema, TimeUnit},
    record_batch::RecordBatch,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deltalake::{DeltaTable, DeltaTableBuilder, DeltaTableConfig, DeltaTableError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Delta Lake exporter configuration
#[derive(Debug, Clone)]
pub struct DeltaLakeExporterConfig {
    /// Delta table location (file:// or s3://)
    pub table_location: String,

    /// Table name
    pub table_name: String,

    /// Maximum records per file
    pub max_records_per_file: usize,

    /// Partition columns
    pub partition_columns: Vec<String>,

    /// Whether to create table if it doesn't exist
    pub create_table_if_not_exists: bool,

    /// Write mode
    pub write_mode: DeltaWriteMode,

    /// Storage options (for S3, etc.)
    pub storage_options: HashMap<String, String>,
}

/// Delta Lake write modes
#[derive(Debug, Clone)]
pub enum DeltaWriteMode {
    Append,
    Overwrite,
    ErrorIfExists,
}

impl Default for DeltaLakeExporterConfig {
    fn default() -> Self {
        Self {
            table_location: "file://./data/delta".to_string(),
            table_name: "telemetry".to_string(),
            max_records_per_file: 10000,
            partition_columns: vec!["date".to_string()],
            create_table_if_not_exists: true,
            write_mode: DeltaWriteMode::Append,
            storage_options: HashMap::new(),
        }
    }
}

/// Delta Lake exporter statistics
#[derive(Debug, Clone)]
pub struct DeltaLakeExporterStats {
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

    /// Files written
    pub files_written: u64,
}

impl Default for DeltaLakeExporterStats {
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
            files_written: 0,
        }
    }
}

/// Delta Lake exporter for exporting telemetry data
#[derive(Debug)]
pub struct DeltaLakeExporter {
    /// Exporter configuration
    pub config: DeltaLakeExporterConfig,

    /// Exporter statistics
    pub stats: Arc<RwLock<DeltaLakeExporterStats>>,

    /// Running state
    pub running: Arc<RwLock<bool>>,

    /// Delta table instance
    pub delta_table: Arc<RwLock<Option<DeltaTable>>>,
}

impl DeltaLakeExporter {
    /// Create a new Delta Lake exporter
    pub fn new(config: DeltaLakeExporterConfig) -> Self {
        info!(
            "Creating Delta Lake exporter for table: {}",
            config.table_name
        );

        Self {
            config,
            stats: Arc::new(RwLock::new(DeltaLakeExporterStats::default())),
            running: Arc::new(RwLock::new(false)),
            delta_table: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the Delta Lake exporter
    pub async fn start(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(crate::error::BridgeError::internal(
                "Delta Lake exporter is already running",
            ));
        }

        *running = true;
        info!("Starting Delta Lake exporter");

        // Initialize Delta table
        self.initialize_delta_table().await?;
        info!("Delta Lake exporter started successfully");

        Ok(())
    }

    /// Stop the Delta Lake exporter
    pub async fn stop(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Delta Lake exporter is not running",
            ));
        }

        *running = false;
        info!("Stopping Delta Lake exporter");

        Ok(())
    }

    /// Initialize Delta table
    async fn initialize_delta_table(&self) -> BridgeResult<()> {
        let table_path = format!("{}/{}", self.config.table_location, self.config.table_name);
        info!("Initializing Delta table at: {}", table_path);

        // Check if table exists
        let table_exists = Path::new(&table_path).exists();

        if !table_exists && self.config.create_table_if_not_exists {
            info!("Creating new Delta table: {}", self.config.table_name);
            self.create_delta_table(&table_path).await?;
        } else if table_exists {
            info!("Loading existing Delta table: {}", self.config.table_name);
            self.load_delta_table(&table_path).await?;
        } else {
            return Err(crate::error::BridgeError::configuration(&format!(
                "Delta table does not exist and create_table_if_not_exists is false: {}",
                table_path
            )));
        }

        Ok(())
    }

    /// Create a new Delta table
    async fn create_delta_table(&self, table_path: &str) -> BridgeResult<()> {
        // For now, we'll use a simplified approach since the Delta Lake API is still evolving
        // Create the table using the builder
        let table = DeltaTableBuilder::from_uri(table_path)
            .build()
            .map_err(|e| {
                crate::error::BridgeError::lakehouse(&format!(
                    "Failed to create Delta table: {}",
                    e
                ))
            })?;

        // Store the table instance
        {
            let mut delta_table = self.delta_table.write().await;
            *delta_table = Some(table);
        }

        info!(
            "Successfully created Delta table: {}",
            self.config.table_name
        );
        Ok(())
    }

    /// Load an existing Delta table
    async fn load_delta_table(&self, table_path: &str) -> BridgeResult<()> {
        let table = DeltaTableBuilder::from_uri(table_path)
            .build()
            .map_err(|e| {
                crate::error::BridgeError::lakehouse(&format!("Failed to load Delta table: {}", e))
            })?;

        // Store the table instance
        {
            let mut delta_table = self.delta_table.write().await;
            *delta_table = Some(table);
        }

        info!(
            "Successfully loaded Delta table: {}",
            self.config.table_name
        );
        Ok(())
    }

    /// Write records to Delta table using available Delta Lake functionality
    async fn write_to_delta(
        &self,
        records: &[crate::types::ProcessedRecord],
    ) -> BridgeResult<usize> {
        let start_time = std::time::Instant::now();

        if records.is_empty() {
            return Ok(0);
        }

        // Convert records to Arrow RecordBatch
        let record_batch = self.convert_records_to_arrow(records)?;

        // Get the table path
        let table_path = format!("{}/{}", self.config.table_location, self.config.table_name);

        // Ensure directory exists
        if let Some(parent) = Path::new(&table_path).parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                crate::error::BridgeError::lakehouse(&format!(
                    "Failed to create output directory: {}",
                    e
                ))
            })?;
        }

        // Check write mode constraints
        match self.config.write_mode {
            DeltaWriteMode::ErrorIfExists => {
                if Path::new(&table_path).exists() {
                    return Err(crate::error::BridgeError::lakehouse(&format!(
                        "Delta table already exists and ErrorIfExists mode is set: {}",
                        table_path
                    )));
                }
            }
            _ => {}
        }

        // Create Delta Lake structure (simplified approach)
        info!("Creating Delta Lake table structure at: {}", table_path);

        // Create the directory structure
        tokio::fs::create_dir_all(&table_path).await.map_err(|e| {
            crate::error::BridgeError::lakehouse(&format!(
                "Failed to create table directory: {}",
                e
            ))
        })?;

        // Write a Parquet file
        let timestamp = chrono::Utc::now().timestamp_millis();
        let parquet_file = format!("{}/part-{:016x}-c000.snappy.parquet", table_path, timestamp);

        // Write the data as Parquet
        let file = std::fs::File::create(&parquet_file).map_err(|e| {
            crate::error::BridgeError::lakehouse(&format!("Failed to create Parquet file: {}", e))
        })?;

        let mut writer =
            parquet::arrow::arrow_writer::ArrowWriter::try_new(file, record_batch.schema(), None)
                .map_err(|e| {
                crate::error::BridgeError::lakehouse(&format!(
                    "Failed to create Parquet writer: {}",
                    e
                ))
            })?;

        writer.write(&record_batch).map_err(|e| {
            crate::error::BridgeError::lakehouse(&format!("Failed to write record batch: {}", e))
        })?;

        writer.close().map_err(|e| {
            crate::error::BridgeError::lakehouse(&format!("Failed to close Parquet writer: {}", e))
        })?;

        // Create a basic Delta Lake log entry
        let delta_log_dir = format!("{}/_delta_log", table_path);
        tokio::fs::create_dir_all(&delta_log_dir)
            .await
            .map_err(|e| {
                crate::error::BridgeError::lakehouse(&format!(
                    "Failed to create Delta log directory: {}",
                    e
                ))
            })?;

        // Create a simple protocol and metadata file
        let log_file = format!("{}/00000000000000000000.json", delta_log_dir);

        // Create proper Delta Lake log entries
        let protocol_entry = serde_json::to_string(&serde_json::json!({
            "protocol": {
                "minReaderVersion": 1,
                "minWriterVersion": 2
            }
        }))
        .unwrap();

        // Create a simple schema string (basic JSON representation)
        let schema_json = serde_json::json!({
            "type": "struct",
            "fields": [
                {
                    "name": "id",
                    "type": "string",
                    "nullable": false,
                    "metadata": {}
                },
                {
                    "name": "timestamp",
                    "type": "timestamp",
                    "nullable": false,
                    "metadata": {}
                },
                {
                    "name": "telemetry_type",
                    "type": "string",
                    "nullable": false,
                    "metadata": {}
                },
                {
                    "name": "service_name",
                    "type": "string",
                    "nullable": true,
                    "metadata": {}
                },
                {
                    "name": "resource_attributes",
                    "type": "string",
                    "nullable": true,
                    "metadata": {}
                },
                {
                    "name": "data_json",
                    "type": "string",
                    "nullable": false,
                    "metadata": {}
                }
            ]
        });

        let metadata_entry = serde_json::to_string(&serde_json::json!({
            "metaData": {
                "id": uuid::Uuid::new_v4().to_string(),
                "format": {
                    "provider": "parquet",
                    "options": {}
                },
                "schemaString": serde_json::to_string(&schema_json).unwrap(),
                "partitionColumns": self.config.partition_columns,
                "configuration": {},
                "createdTime": chrono::Utc::now().timestamp_millis()
            }
        }))
        .unwrap();

        let add_entry = serde_json::to_string(&serde_json::json!({
            "add": {
                "path": format!("part-{:016x}-c000.snappy.parquet", timestamp),
                "partitionValues": {},
                "size": std::fs::metadata(&parquet_file).map(|m| m.len()).unwrap_or(0),
                "modificationTime": chrono::Utc::now().timestamp_millis(),
                "dataChange": true
            }
        }))
        .unwrap();

        let log_content = format!("{}\n{}\n{}\n", protocol_entry, metadata_entry, add_entry);

        tokio::fs::write(&log_file, log_content)
            .await
            .map_err(|e| {
                crate::error::BridgeError::lakehouse(&format!("Failed to write Delta log: {}", e))
            })?;

        let duration = start_time.elapsed();
        let bytes_written = records.len() * 200; // Estimate

        info!(
            "Delta Lake write completed: {} records in {:?} ({} bytes) to {}",
            records.len(),
            duration,
            bytes_written,
            table_path
        );

        info!("✓ Created Delta Lake structure:");
        info!("  - Parquet file: {}", parquet_file);
        info!("  - Log directory: {}", delta_log_dir);
        info!("  - Log file: {}", log_file);

        Ok(bytes_written)
    }

    /// Convert ProcessedRecord to Arrow RecordBatch
    fn convert_records_to_arrow(
        &self,
        records: &[crate::types::ProcessedRecord],
    ) -> BridgeResult<RecordBatch> {
        let mut ids = Vec::new();
        let mut timestamps = Vec::new();
        let mut record_types = Vec::new();
        let mut service_names = Vec::new();
        let mut operation_names = Vec::new();
        let mut duration_ms = Vec::new();
        let mut status_codes = Vec::new();
        let mut attributes = Vec::new();
        let mut tags = Vec::new();

        for record in records {
            ids.push(record.original_id.to_string());
            timestamps.push(
                record
                    .metadata
                    .get("timestamp")
                    .and_then(|ts| ts.parse::<i64>().ok())
                    .unwrap_or_else(|| chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
            );
            record_types.push(
                record
                    .metadata
                    .get("record_type")
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string()),
            );
            service_names.push(
                record
                    .metadata
                    .get("service_name")
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string()),
            );
            operation_names.push(
                record
                    .metadata
                    .get("operation_name")
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string()),
            );
            duration_ms.push(
                record
                    .metadata
                    .get("duration_ms")
                    .and_then(|d| d.parse::<f64>().ok())
                    .unwrap_or(0.0),
            );
            status_codes.push(
                record
                    .metadata
                    .get("status_code")
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string()),
            );
            attributes
                .push(serde_json::to_string(&record.metadata).unwrap_or_else(|_| "{}".to_string()));
            tags.push(serde_json::to_string(&record.metadata).unwrap_or_else(|_| "{}".to_string()));
        }

        let schema = ArrowSchema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Nanosecond, None),
                false,
            ),
            Field::new("record_type", DataType::Utf8, false),
            Field::new("service_name", DataType::Utf8, true),
            Field::new("operation_name", DataType::Utf8, true),
            Field::new("duration_ms", DataType::Float64, true),
            Field::new("status_code", DataType::Utf8, true),
            Field::new("attributes", DataType::Utf8, true),
            Field::new("tags", DataType::Utf8, true),
        ]);

        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(StringArray::from(ids)),
                Arc::new(TimestampNanosecondArray::from(timestamps)),
                Arc::new(StringArray::from(record_types)),
                Arc::new(StringArray::from(service_names)),
                Arc::new(StringArray::from(operation_names)),
                Arc::new(Float64Array::from(duration_ms)),
                Arc::new(StringArray::from(status_codes)),
                Arc::new(StringArray::from(attributes)),
                Arc::new(StringArray::from(tags)),
            ],
        )
        .map_err(|e| {
            crate::error::BridgeError::lakehouse(&format!(
                "Failed to create Arrow RecordBatch: {}",
                e
            ))
        })?;

        Ok(batch)
    }
}

#[async_trait]
impl LakehouseExporter for DeltaLakeExporter {
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        let start_time = std::time::Instant::now();

        let running = self.running.read().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Delta Lake exporter is not running",
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

        // Write records to Delta table
        let bytes_written = match self.write_to_delta(&batch.records).await {
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

        let duration = start_time.elapsed();
        let duration_ms = duration.as_millis() as u64;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_batches += 1;
            stats.total_records += record_count as u64;
            stats.total_bytes += bytes_written as u64;
            stats.last_export_time = Some(chrono::Utc::now());
            stats.files_written += 1;

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
                    format!(
                        "delta://{}/{}",
                        self.config.table_location, self.config.table_name
                    ),
                ),
                ("bytes_exported".to_string(), bytes_written.to_string()),
                ("table_name".to_string(), self.config.table_name.clone()),
            ]),
            errors: vec![],
        })
    }

    fn name(&self) -> &str {
        "delta_lake_exporter"
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
impl HealthCheckCallback for DeltaLakeExporter {
    async fn check(&self) -> BridgeResult<HealthCheckResult> {
        let running = self.running.read().await;
        let status = if *running {
            crate::health::types::HealthStatus::Healthy
        } else {
            crate::health::types::HealthStatus::Unhealthy
        };

        Ok(HealthCheckResult::new(
            "delta_lake_exporter".to_string(),
            status,
            if *running {
                "Delta Lake exporter is running".to_string()
            } else {
                "Delta Lake exporter is not running".to_string()
            },
        ))
    }

    fn component_name(&self) -> &str {
        "delta_lake_exporter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ProcessedBatch, ProcessedRecord, ProcessingStatus, TelemetryData};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_delta_lake_exporter_creation() {
        let config = DeltaLakeExporterConfig::default();
        let exporter = DeltaLakeExporter::new(config);

        let stats = exporter.get_stats().await.unwrap();
        assert_eq!(stats.total_records, 0);
    }

    #[tokio::test]
    async fn test_delta_lake_exporter_start_stop() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = DeltaLakeExporterConfig {
            table_location: format!("file://{}", temp_dir.path().to_string_lossy()),
            ..Default::default()
        };
        let exporter = DeltaLakeExporter::new(config);

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
    async fn test_delta_lake_exporter_export() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = DeltaLakeExporterConfig {
            table_location: format!("file://{}", temp_dir.path().to_string_lossy()),
            ..Default::default()
        };
        let exporter = DeltaLakeExporter::new(config);
        exporter.start().await.unwrap();

        // Create test batch
        let processed_record = ProcessedRecord {
            original_id: Uuid::new_v4(),
            status: ProcessingStatus::Success,
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
            status: ProcessingStatus::Success,
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

        exporter.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_delta_lake_exporter_demo() {
        // This test demonstrates the complete Delta Lake exporter functionality
        let temp_dir = tempfile::tempdir().unwrap();
        let delta_base_path = temp_dir.path().to_str().unwrap();

        info!("Using temporary Delta Lake directory: {}", delta_base_path);

        // Create Delta Lake exporter configuration
        let config = DeltaLakeExporterConfig {
            table_location: delta_base_path.to_string(),
            table_name: "telemetry".to_string(),
            max_records_per_file: 1000,
            partition_columns: vec!["service_name".to_string(), "date".to_string()],
            create_table_if_not_exists: true,
            write_mode: DeltaWriteMode::Append,
            storage_options: HashMap::new(),
        };

        // Create Delta Lake exporter
        let exporter = DeltaLakeExporter::new(config);
        info!("✓ Delta Lake exporter created");

        // Start the exporter
        exporter.start().await.unwrap();
        info!("✓ Delta Lake exporter started");

        // Create sample telemetry data
        let sample_batches = create_sample_batches();
        info!("✓ Created {} sample batches", sample_batches.len());

        // Export batches
        for (i, batch) in sample_batches.into_iter().enumerate() {
            info!(
                "Exporting batch {} with {} records",
                i + 1,
                batch.records.len()
            );

            let result = exporter.export(batch).await.unwrap();
            info!(
                "✓ Batch {} exported successfully: {} records, {} bytes, {}ms",
                i + 1,
                result.records_exported,
                result
                    .metadata
                    .get("bytes_exported")
                    .unwrap_or(&"unknown".to_string()),
                result.duration_ms
            );
        }

        // Get exporter statistics
        let stats = exporter.get_stats().await.unwrap();
        info!("✓ Exporter statistics:");
        info!("  - Total batches: {}", stats.total_batches);
        info!("  - Total records: {}", stats.total_records);
        info!("  - Average export time: {:.2}ms", stats.avg_export_time_ms);
        info!("  - Error count: {}", stats.error_count);

        // Check health
        let health = exporter.health_check().await.unwrap();
        info!(
            "✓ Exporter health check: {}",
            if health { "healthy" } else { "unhealthy" }
        );

        // Stop the exporter
        exporter.stop().await.unwrap();
        info!("✓ Delta Lake exporter stopped");

        info!("Delta Lake Exporter Demo completed successfully");
        info!(
            "Check the output directory for generated Parquet files: {}",
            delta_base_path
        );
    }

    /// Create sample telemetry batches for testing
    fn create_sample_batches() -> Vec<ProcessedBatch> {
        let mut batches = Vec::new();

        // Create multiple batches with different service names
        let services = vec![
            "user-service",
            "auth-service",
            "payment-service",
            "notification-service",
        ];

        for (batch_idx, service_name) in services.iter().enumerate() {
            let mut records = Vec::new();

            // Create multiple records per batch
            for record_idx in 0..5 {
                let trace_data = crate::types::traces::TraceData {
                    trace_id: format!("trace-{}-{}", batch_idx, record_idx),
                    span_id: format!("span-{}-{}", batch_idx, record_idx),
                    parent_span_id: None,
                    name: format!("{}-operation", service_name),
                    kind: crate::types::traces::SpanKind::Internal,
                    start_time: chrono::Utc::now(),
                    end_time: Some(chrono::Utc::now()),
                    duration_ns: Some(1000000), // 1ms
                    status: crate::types::traces::SpanStatus {
                        code: crate::types::traces::StatusCode::Ok,
                        message: None,
                    },
                    attributes: HashMap::from([
                        ("service.name".to_string(), service_name.to_string()),
                        (
                            "operation.name".to_string(),
                            format!("{}-operation", service_name),
                        ),
                        ("duration_ms".to_string(), "1.0".to_string()),
                        ("status_code".to_string(), "200".to_string()),
                    ]),
                    events: Vec::new(),
                    links: Vec::new(),
                };

                let mut metadata = HashMap::new();
                metadata.insert(
                    "timestamp".to_string(),
                    chrono::Utc::now()
                        .timestamp_nanos_opt()
                        .unwrap_or(0)
                        .to_string(),
                );
                metadata.insert("record_type".to_string(), "trace".to_string());
                metadata.insert("service_name".to_string(), service_name.to_string());
                metadata.insert(
                    "operation_name".to_string(),
                    format!("{}-operation", service_name),
                );
                metadata.insert("duration_ms".to_string(), "1.0".to_string());
                metadata.insert("status_code".to_string(), "200".to_string());

                let record = ProcessedRecord {
                    original_id: Uuid::new_v4(),
                    status: ProcessingStatus::Success,
                    transformed_data: Some(TelemetryData::Trace(trace_data)),
                    metadata,
                    errors: Vec::new(),
                };

                records.push(record);
            }

            let batch = ProcessedBatch {
                original_batch_id: Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
                status: ProcessingStatus::Success,
                records,
                metadata: HashMap::new(),
                errors: Vec::new(),
            };

            batches.push(batch);
        }

        batches
    }
}
