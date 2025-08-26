//! Delta Lake exporter main implementation

use crate::error::BridgeResult;
use crate::traits::exporter::LakehouseExporter;
use crate::types::{ExportResult, ProcessedBatch};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deltalake::{DeltaTable, DeltaTableBuilder, DeltaTableConfig, DeltaTableError};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::config::{DeltaLakeExporterConfig, DeltaLakeExporterStats, DeltaWriteMode};
use super::converter::{convert_records_to_arrow, create_add_entry, create_metadata_entry, create_protocol_entry};

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
        let record_batch = convert_records_to_arrow(records)?;

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
        let protocol_entry = create_protocol_entry();
        let metadata_entry = create_metadata_entry(&Uuid::new_v4().to_string(), &self.config.partition_columns);
        let add_entry = create_add_entry(&format!("part-{:016x}-c000.snappy.parquet", timestamp), std::fs::metadata(&parquet_file).map(|m| m.len()).unwrap_or(0));

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

    /// Get exporter statistics
    pub async fn get_stats(&self) -> DeltaLakeExporterStats {
        self.stats.read().await.clone()
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
