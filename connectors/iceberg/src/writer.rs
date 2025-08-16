//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Iceberg writer implementation
//!
//! This module provides the Apache Iceberg writer that implements
//! the LakehouseWriter trait for writing telemetry data to Iceberg tables.

use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use chrono::Utc;
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info};

use bridge_core::error::BridgeResult;
use bridge_core::traits::{LakehouseWriter, WriterStats};
use bridge_core::types::{LogsBatch, MetricsBatch, TelemetryBatch, TracesBatch, WriteResult};

use crate::catalog::IcebergCatalog;
use crate::config::IcebergConfig;
use crate::error::{IcebergError, IcebergResult};
use crate::schema::IcebergSchema;

/// Apache Iceberg writer implementation
#[derive(Clone)]
pub struct IcebergWriter {
    /// Iceberg configuration
    config: IcebergConfig,
    /// Catalog instance
    catalog: Arc<IcebergCatalog>,
    /// Schema instance
    schema: Arc<IcebergSchema>,
    /// Statistics
    stats: WriterStats,
    /// Pending batches for writing
    pending_batches: Vec<RecordBatch>,
    /// Last flush time
    last_flush_time: Instant,
}

impl IcebergWriter {
    /// Create a new Apache Iceberg writer
    pub async fn new(
        config: IcebergConfig,
        catalog: Arc<IcebergCatalog>,
        schema: Arc<IcebergSchema>,
    ) -> IcebergResult<Self> {
        info!(
            "Creating Apache Iceberg writer for table: {}",
            config.table.table_name
        );

        Ok(Self {
            config,
            catalog,
            schema,
            stats: WriterStats {
                total_writes: 0,
                total_records: 0,
                writes_per_minute: 0,
                records_per_minute: 0,
                avg_write_time_ms: 0.0,
                error_count: 0,
                last_write_time: None,
            },
            pending_batches: Vec::new(),
            last_flush_time: Instant::now(),
        })
    }

    /// Convert telemetry batch to Arrow record batch
    async fn convert_to_record_batch(&self, batch: &TelemetryBatch) -> IcebergResult<RecordBatch> {
        // This is a simplified conversion - in a real implementation,
        // you would properly convert the telemetry data to Arrow format
        // based on the Iceberg schema

        debug!(
            "Converting telemetry batch with {} records to Arrow record batch",
            batch.records.len()
        );

        // For now, return an empty record batch
        // This would be replaced with actual Arrow conversion logic
        Err(IcebergError::not_implemented(
            "Telemetry batch conversion not yet implemented".to_string(),
        ))
    }

    /// Write record batches to Iceberg table
    async fn write_record_batches(&mut self, batches: Vec<RecordBatch>) -> IcebergResult<()> {
        if batches.is_empty() {
            return Ok(());
        }

        let start_time = Instant::now();
        let total_records: usize = batches.iter().map(|batch| batch.num_rows()).sum();

        info!(
            "Writing {} record batches with {} total records to Iceberg table",
            batches.len(),
            total_records
        );

        // Create a new transaction
        let mut transaction = self
            .catalog
            .create_transaction(&self.config.table.table_name)
            .await?;

        // Write each batch
        for batch in batches {
            // Convert batch to Parquet format
            let parquet_data = self.convert_batch_to_parquet(batch).await?;

            // Add file to transaction
            transaction.add_file(parquet_data).await?;
        }

        // Commit transaction
        transaction.commit().await?;

        // Update statistics
        let write_time = start_time.elapsed();
        self.stats.total_writes += 1;
        self.stats.total_records += total_records as u64;
        self.stats.last_write_time = Some(Utc::now());

        // Update average write time
        let total_time_ms = self.stats.avg_write_time_ms * (self.stats.total_writes - 1) as f64
            + write_time.as_millis() as f64;
        self.stats.avg_write_time_ms = total_time_ms / self.stats.total_writes as f64;

        info!(
            "Successfully wrote {} records to Iceberg table in {:?}",
            total_records, write_time
        );

        Ok(())
    }

    /// Convert Arrow record batch to Parquet format
    async fn convert_batch_to_parquet(&self, batch: RecordBatch) -> IcebergResult<Vec<u8>> {
        // This is a placeholder implementation
        // In a real implementation, you would convert the Arrow batch to Parquet format

        let mut buffer = Vec::new();
        let props = WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build();

        let mut writer =
            ArrowWriter::try_new(&mut buffer, batch.schema(), Some(props)).map_err(|e| {
                IcebergError::conversion(format!("Failed to create Parquet writer: {}", e))
            })?;

        writer.write(&batch).map_err(|e| {
            IcebergError::conversion(format!("Failed to write batch to Parquet: {}", e))
        })?;

        writer.close().map_err(|e| {
            IcebergError::conversion(format!("Failed to close Parquet writer: {}", e))
        })?;

        Ok(buffer)
    }

    /// Check if we should flush based on time or batch size
    fn should_flush(&self) -> bool {
        let time_since_flush = self.last_flush_time.elapsed();
        let batch_size_threshold = self.config.writer.batch_size;
        let time_threshold = Duration::from_millis(self.config.writer.flush_interval_ms);

        self.pending_batches.len() >= batch_size_threshold || time_since_flush >= time_threshold
    }
}

#[async_trait]
impl LakehouseWriter for IcebergWriter {
    async fn write_metrics(&self, metrics: MetricsBatch) -> BridgeResult<WriteResult> {
        let start_time = Instant::now();

        // Convert metrics to telemetry batch
        let batch = TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "iceberg-writer".to_string(),
            size: metrics.metrics.len(),
            records: metrics
                .metrics
                .into_iter()
                .map(|record| bridge_core::types::TelemetryRecord {
                    id: uuid::Uuid::new_v4(),
                    timestamp: record.timestamp,
                    record_type: bridge_core::types::TelemetryType::Metric,
                    data: bridge_core::types::TelemetryData::Metric(record),
                    attributes: HashMap::new(),
                    tags: HashMap::new(),
                    resource: None,
                    service: None,
                })
                .collect(),
            metadata: HashMap::new(),
        };

        let result = self.write_batch(batch).await;

        match &result {
            Ok(_) => {
                let write_time = start_time.elapsed();
                info!("Successfully wrote metrics batch in {:?}", write_time);
            }
            Err(e) => {
                error!("Failed to write metrics batch: {}", e);
            }
        }

        result
    }

    async fn write_traces(&self, traces: TracesBatch) -> BridgeResult<WriteResult> {
        let start_time = Instant::now();

        // Convert traces to telemetry batch
        let batch = TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "iceberg-writer".to_string(),
            size: traces.traces.len(),
            records: traces
                .traces
                .into_iter()
                .map(|record| bridge_core::types::TelemetryRecord {
                    id: uuid::Uuid::new_v4(),
                    timestamp: record.start_time,
                    record_type: bridge_core::types::TelemetryType::Trace,
                    data: bridge_core::types::TelemetryData::Trace(record),
                    attributes: HashMap::new(),
                    tags: HashMap::new(),
                    resource: None,
                    service: None,
                })
                .collect(),
            metadata: HashMap::new(),
        };

        let result = self.write_batch(batch).await;

        match &result {
            Ok(_) => {
                let write_time = start_time.elapsed();
                info!("Successfully wrote traces batch in {:?}", write_time);
            }
            Err(e) => {
                error!("Failed to write traces batch: {}", e);
            }
        }

        result
    }

    async fn write_logs(&self, logs: LogsBatch) -> BridgeResult<WriteResult> {
        let start_time = Instant::now();

        // Convert logs to telemetry batch
        let batch = TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "iceberg-writer".to_string(),
            size: logs.logs.len(),
            records: logs
                .logs
                .into_iter()
                .map(|record| bridge_core::types::TelemetryRecord {
                    id: uuid::Uuid::new_v4(),
                    timestamp: record.timestamp,
                    record_type: bridge_core::types::TelemetryType::Log,
                    data: bridge_core::types::TelemetryData::Log(record),
                    attributes: HashMap::new(),
                    tags: HashMap::new(),
                    resource: None,
                    service: None,
                })
                .collect(),
            metadata: HashMap::new(),
        };

        let result = self.write_batch(batch).await;

        match &result {
            Ok(_) => {
                let write_time = start_time.elapsed();
                info!("Successfully wrote logs batch in {:?}", write_time);
            }
            Err(e) => {
                error!("Failed to write logs batch: {}", e);
            }
        }

        result
    }

    async fn write_batch(&self, batch: TelemetryBatch) -> BridgeResult<WriteResult> {
        let start_time = Instant::now();

        // Convert to record batch
        let record_batch = self.convert_to_record_batch(&batch).await.map_err(|e| {
            bridge_core::error::BridgeError::lakehouse_with_source(
                "Failed to convert telemetry batch to record batch",
                e,
            )
        })?;

        // Write to Iceberg
        let mut writer = IcebergWriter {
            config: self.config.clone(),
            catalog: self.catalog.clone(),
            schema: self.schema.clone(),
            stats: self.stats.clone(),
            pending_batches: vec![record_batch],
            last_flush_time: self.last_flush_time,
        };

        writer
            .write_record_batches(writer.pending_batches.clone())
            .await
            .map_err(|e| {
                bridge_core::error::BridgeError::lakehouse_with_source(
                    "Failed to write record batches to Iceberg",
                    e,
                )
            })?;

        let write_time = start_time.elapsed();

        Ok(WriteResult {
            timestamp: Utc::now(),
            status: bridge_core::types::WriteStatus::Success,
            records_written: batch.records.len(),
            records_failed: 0,
            duration_ms: write_time.as_millis() as u64,
            metadata: HashMap::new(),
            errors: Vec::new(),
        })
    }

    async fn flush(&self) -> BridgeResult<()> {
        if !self.pending_batches.is_empty() {
            let mut writer = IcebergWriter {
                config: self.config.clone(),
                catalog: self.catalog.clone(),
                schema: self.schema.clone(),
                stats: self.stats.clone(),
                pending_batches: self.pending_batches.clone(),
                last_flush_time: self.last_flush_time,
            };

            writer
                .write_record_batches(writer.pending_batches.clone())
                .await
                .map_err(|e| {
                    bridge_core::error::BridgeError::lakehouse_with_source(
                        "Failed to flush pending batches",
                        e,
                    )
                })?;
        }

        Ok(())
    }

    async fn get_stats(&self) -> BridgeResult<WriterStats> {
        Ok(self.stats.clone())
    }

    async fn close(&self) -> BridgeResult<()> {
        info!("Closing Apache Iceberg writer");

        // Flush any pending batches
        self.flush().await?;

        info!("Apache Iceberg writer closed successfully");
        Ok(())
    }
}
