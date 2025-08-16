//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Iceberg exporter implementation
//! 
//! This module provides a real Apache Iceberg exporter that uses the actual
//! Iceberg writer to export telemetry data to Iceberg tables.

use std::sync::{Arc, RwLock};
use std::time::Instant;
use async_trait::async_trait;
use tracing::{debug, error, info, warn};
use bridge_core::traits::{LakehouseExporter, ExporterStats, LakehouseWriter};
use bridge_core::types::{TelemetryBatch, ProcessedBatch, ExportResult, ExportError, ExportStatus, MetricsBatch, TracesBatch, LogsBatch, TelemetryData};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use bridge_core::error::BridgeResult;

use crate::config::IcebergConfig;
use crate::error::IcebergResult;
use crate::writer::IcebergWriter;
use crate::catalog::IcebergCatalog;
use crate::schema::IcebergSchema;

/// Real Apache Iceberg exporter implementation
pub struct RealIcebergExporter {
    /// Iceberg configuration
    config: IcebergConfig,
    /// Iceberg writer instance
    writer: Arc<IcebergWriter>,
    /// Exporter state
    running: Arc<RwLock<bool>>,
    /// Exporter statistics
    stats: Arc<RwLock<ExporterStats>>,
}

impl RealIcebergExporter {
    /// Create a new Apache Iceberg exporter
    pub async fn new(config: IcebergConfig) -> Self {
        let stats = ExporterStats {
            total_batches: 0,
            total_records: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            avg_export_time_ms: 0.0,
            error_count: 0,
            last_export_time: None,
        };

        // Create catalog and schema for the writer
        let catalog = Arc::new(IcebergCatalog::new(config.catalog.clone()).await.unwrap());
        let schema = Arc::new(IcebergSchema::new(config.schema.clone()).await.unwrap());
        let writer = IcebergWriter::new(config.clone(), catalog, schema).await.unwrap();
        Self {
            writer: Arc::new(writer),
            config,
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
        }
    }

    /// Initialize the exporter
    pub async fn initialize(&mut self) -> IcebergResult<()> {
        info!("Initializing Apache Iceberg exporter: {}", self.config.table_name());
        
        // The writer is already initialized in the constructor
        
        // Mark exporter as running
        {
            let mut running = self.running.write().unwrap();
            *running = true;
        }
        
        info!("Apache Iceberg exporter initialized successfully: {}", self.config.table_name());
        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &IcebergConfig {
        &self.config
    }

    /// Check if the exporter is running
    pub fn is_running(&self) -> bool {
        *self.running.read().unwrap()
    }

    /// Get exporter name
    pub fn name(&self) -> &str {
        &self.config.table.table_name
    }

    /// Get exporter version
    pub fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    /// Health check for the exporter
    pub async fn health_check(&self) -> BridgeResult<bool> {
        if !self.is_running() {
            return Ok(false);
        }

        // Check if writer is healthy - for now just return true since writer doesn't have health_check
        let writer_healthy = true;
        
        Ok(writer_healthy)
    }
}

#[async_trait]
impl LakehouseExporter for RealIcebergExporter {
    fn name(&self) -> &str {
        &self.config.table.table_name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        // For now just return true since writer doesn't have health_check
        Ok(self.is_running())
    }
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        // Check if exporter is running
        if !self.is_running() {
            return Err(bridge_core::error::BridgeError::export(
                "Apache Iceberg exporter is not running"
            ));
        }

        let start_time = Instant::now();
        info!("Real Apache Iceberg exporter exporting batch with {} records", batch.records.len());

        let mut total_records = 0u64;
        let mut errors = Vec::new();

        // Process each record in the batch
        for record in &batch.records {
            if let Some(transformed_data) = &record.transformed_data {
                match transformed_data {
                    TelemetryData::Metric(metric_data) => {
                        // Create a metrics batch for this single metric
                        let metrics_batch = MetricsBatch {
                            id: record.original_id,
                            timestamp: record.metadata.get("timestamp")
                                .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                                .unwrap_or_else(|| chrono::Utc::now()),
                            metrics: vec![metric_data.clone()],
                            metadata: record.metadata.clone(),
                        };
                        
                        match self.writer.write_metrics(metrics_batch).await {
                            Ok(result) => {
                                total_records += 1;
                                debug!("Successfully exported metric record to Apache Iceberg");
                            }
                            Err(e) => {
                                let error = ExportError {
                                    code: "iceberg_write_error".to_string(),
                                    message: format!("Failed to export metric: {}", e),
                                    details: None,
                                };
                                errors.push(error);
                                error!("Failed to export metric to Apache Iceberg: {}", e);
                            }
                        }
                    }
                    TelemetryData::Trace(trace_data) => {
                        // Create a traces batch for this single trace
                        let traces_batch = TracesBatch {
                            id: record.original_id,
                            timestamp: record.metadata.get("timestamp")
                                .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                                .unwrap_or_else(|| chrono::Utc::now()),
                            traces: vec![trace_data.clone()],
                            metadata: record.metadata.clone(),
                        };
                        
                        match self.writer.write_traces(traces_batch).await {
                            Ok(result) => {
                                total_records += 1;
                                debug!("Successfully exported trace record to Apache Iceberg");
                            }
                            Err(e) => {
                                let error = ExportError {
                                    code: "iceberg_write_error".to_string(),
                                    message: format!("Failed to export trace: {}", e),
                                    details: None,
                                };
                                errors.push(error);
                                error!("Failed to export trace to Apache Iceberg: {}", e);
                            }
                        }
                    }
                    TelemetryData::Log(log_data) => {
                        // Create a logs batch for this single log
                        let logs_batch = LogsBatch {
                            id: record.original_id,
                            timestamp: record.metadata.get("timestamp")
                                .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                                .unwrap_or_else(|| chrono::Utc::now()),
                            logs: vec![log_data.clone()],
                            metadata: record.metadata.clone(),
                        };
                        
                        match self.writer.write_logs(logs_batch).await {
                            Ok(result) => {
                                total_records += 1;
                                debug!("Successfully exported log record to Apache Iceberg");
                            }
                            Err(e) => {
                                let error = ExportError {
                                    code: "iceberg_write_error".to_string(),
                                    message: format!("Failed to export log: {}", e),
                                    details: None,
                                };
                                errors.push(error);
                                error!("Failed to export log to Apache Iceberg: {}", e);
                            }
                        }
                    }
                    _ => {
                        // Skip other data types for now
                        debug!("Skipping unsupported telemetry data type");
                    }
                }
            }
        }

        let export_duration = start_time.elapsed();

        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_batches += 1;
            stats.total_records += total_records;
            stats.error_count += errors.len() as u64;
            stats.last_export_time = Some(chrono::Utc::now());
        }

        if errors.is_empty() {
            info!("Real Apache Iceberg exporter successfully exported {} records in {:?}",
                  total_records, export_duration);
        } else {
            warn!("Real Apache Iceberg exporter failed to export some records: {} errors",
                  errors.len());
        }

        Ok(ExportResult {
            timestamp: chrono::Utc::now(),
            status: if errors.is_empty() { ExportStatus::Success } else { ExportStatus::Partial },
            records_exported: total_records as usize,
            records_failed: errors.len(),
            duration_ms: export_duration.as_millis() as u64,
            metadata: HashMap::new(),
            errors,
        })
    }

    async fn get_stats(&self) -> BridgeResult<ExporterStats> {
        let stats = self.stats.read().unwrap();
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Real Apache Iceberg exporter shutting down");

        // Mark exporter as not running
        {
            let mut running = self.running.write().unwrap();
            *running = false;
        }

        // Close the writer
        let _ = self.writer.close().await;

        info!("Real Apache Iceberg exporter shutdown completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IcebergConfig;
    use bridge_core::types::{MetricData, MetricValue};

    #[tokio::test]
    async fn test_real_iceberg_exporter_creation() {
        let config = IcebergConfig {
            table: crate::config::IcebergTableConfig {
                table_name: "test_table".to_string(),
                table_format_version: 2,
                partition_columns: vec!["year".to_string(), "month".to_string()],
                compression: "snappy".to_string(),
                properties: HashMap::new(),
            },
            storage: crate::config::IcebergStorageConfig {
                storage_path: "s3://iceberg-warehouse".to_string(),
                storage_type: "s3".to_string(),
                options: HashMap::new(),
            },
            catalog: crate::config::IcebergCatalogConfig {
                catalog_type: "hive".to_string(),
                catalog_uri: Some("thrift://localhost:9083".to_string()),
                warehouse_location: "s3://iceberg-warehouse".to_string(),
                properties: HashMap::new(),
            },
            ..Default::default()
        };

        let exporter = RealIcebergExporter::new(config).await;
        assert_eq!(exporter.name(), "test_table");
        assert_eq!(exporter.version(), env!("CARGO_PKG_VERSION"));
        assert!(!exporter.is_running());
    }

    #[tokio::test]
    async fn test_real_iceberg_exporter_initialization() {
        let config = IcebergConfig {
            table: crate::config::IcebergTableConfig {
                table_name: "test_table".to_string(),
                table_format_version: 2,
                partition_columns: vec!["year".to_string(), "month".to_string()],
                compression: "snappy".to_string(),
                properties: HashMap::new(),
            },
            storage: crate::config::IcebergStorageConfig {
                storage_path: "s3://iceberg-warehouse".to_string(),
                storage_type: "s3".to_string(),
                options: HashMap::new(),
            },
            catalog: crate::config::IcebergCatalogConfig {
                catalog_type: "hive".to_string(),
                catalog_uri: Some("thrift://localhost:9083".to_string()),
                warehouse_location: "s3://iceberg-warehouse".to_string(),
                properties: HashMap::new(),
            },
            ..Default::default()
        };

        let mut exporter = RealIcebergExporter::new(config).await;
        let result = exporter.initialize().await;
        assert!(result.is_ok());

        // Check that exporter is running after initialization
        assert!(exporter.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_real_iceberg_exporter_export() {
        let config = IcebergConfig {
            table: crate::config::IcebergTableConfig {
                table_name: "test_table".to_string(),
                table_format_version: 2,
                partition_columns: vec!["year".to_string(), "month".to_string()],
                compression: "snappy".to_string(),
                properties: HashMap::new(),
            },
            storage: crate::config::IcebergStorageConfig {
                storage_path: "s3://iceberg-warehouse".to_string(),
                storage_type: "s3".to_string(),
                options: HashMap::new(),
            },
            catalog: crate::config::IcebergCatalogConfig {
                catalog_type: "hive".to_string(),
                catalog_uri: Some("thrift://localhost:9083".to_string()),
                warehouse_location: "s3://iceberg-warehouse".to_string(),
                properties: HashMap::new(),
            },
            ..Default::default()
        };

        let mut exporter = RealIcebergExporter::new(config).await;
        exporter.initialize().await.unwrap();

        // Create a test batch
        let batch = ProcessedBatch {
            original_batch_id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            status: bridge_core::types::ProcessingStatus::Success,
            records: vec![
                bridge_core::types::ProcessedRecord {
                    original_id: uuid::Uuid::new_v4(),
                    status: bridge_core::types::ProcessingStatus::Success,
                    transformed_data: Some(TelemetryData::Metric(MetricData {
                        name: "test_metric".to_string(),
                        description: None,
                        unit: None,
                        metric_type: bridge_core::types::MetricType::Gauge,
                        value: MetricValue::Gauge(42.0),
                        labels: HashMap::new(),
                        timestamp: chrono::Utc::now(),
                    })),
                    metadata: HashMap::new(),
                    errors: vec![],
                }
            ],
            metadata: HashMap::new(),
            errors: vec![],
        };

        let result = exporter.export(batch).await;
        assert!(result.is_ok());

        let export_result = result.unwrap();
        // In test environment, the writer is not connected to a real Iceberg table,
        // so we expect 0 records to be exported
        assert_eq!(export_result.records_exported, 0);
        // We expect some errors since the writer is not connected to a real table
        assert!(export_result.errors.len() > 0);
    }
}
