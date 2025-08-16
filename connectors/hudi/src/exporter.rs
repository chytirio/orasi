//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Hudi exporter implementation
//! 
//! This module provides a real Apache Hudi exporter that uses the actual
//! Hudi writer to export telemetry data to Hudi tables.

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use async_trait::async_trait;
use tracing::{debug, error, info, warn};
use bridge_core::traits::{LakehouseExporter, ExporterStats, LakehouseWriter};
use bridge_core::types::{TelemetryBatch, ExportResult, ExportError, ProcessedBatch};
use bridge_core::error::BridgeResult;

use crate::config::HudiConfig;
use crate::error::{HudiError, HudiResult};
use crate::writer::HudiWriter;

/// Real Apache Hudi exporter implementation
pub struct RealHudiExporter {
    /// Hudi configuration
    config: HudiConfig,
    /// Hudi writer instance
    writer: Arc<HudiWriter>,
    /// Exporter state
    running: Arc<RwLock<bool>>,
    /// Exporter statistics
    stats: Arc<RwLock<ExporterStats>>,
}

impl RealHudiExporter {
    /// Create a new Apache Hudi exporter
    pub async fn new(config: HudiConfig) -> Self {
        let stats = ExporterStats {
            total_batches: 0,
            total_records: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            avg_export_time_ms: 0.0,
            error_count: 0,
            last_export_time: None,
        };

        Self {
            writer: Arc::new(HudiWriter::new(config.clone()).await.unwrap()),
            config,
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
        }
    }

    /// Initialize the exporter
    pub async fn initialize(&mut self) -> HudiResult<()> {
        info!("Initializing Apache Hudi exporter: {}", self.config.table.table_name);
        
        // Initialize the Hudi writer
        self.writer.initialize().await?;
        
        // Mark exporter as running
        {
            let mut running = self.running.write().unwrap();
            *running = true;
        }
        
        info!("Apache Hudi exporter initialized successfully: {}", self.config.table.table_name);
        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &HudiConfig {
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

        // For now, just check if writer is initialized
        let writer_healthy = self.writer.is_initialized();
        
        Ok(writer_healthy)
    }
}

#[async_trait]
impl LakehouseExporter for RealHudiExporter {
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        // Check if exporter is running
        if !self.is_running() {
            return Err(bridge_core::error::BridgeError::export(
                "Apache Hudi exporter is not running"
            ));
        }

        let start_time = Instant::now();
        info!("Real Apache Hudi exporter exporting batch with {} records", batch.records.len());

        let mut total_records = 0u64;
        let mut errors = Vec::new();

        // Export the processed batch - convert ProcessedBatch to TelemetryBatch
        let telemetry_batch = bridge_core::types::TelemetryBatch {
            id: batch.original_batch_id,
            timestamp: batch.timestamp,
            source: "hudi-exporter".to_string(),
            size: batch.records.len(),
            records: batch.records.iter().filter_map(|record| {
                match record.status {
                    bridge_core::types::ProcessingStatus::Success => {
                        Some(bridge_core::types::TelemetryRecord {
                        id: record.original_id,
                        timestamp: batch.timestamp,
                        record_type: bridge_core::types::TelemetryType::Metric, // Default to metric
                        data: record.transformed_data.clone().unwrap_or_else(|| {
                            bridge_core::types::TelemetryData::Metric(bridge_core::types::MetricData {
                                name: "unknown".to_string(),
                                description: None,
                                unit: None,
                                metric_type: bridge_core::types::MetricType::Gauge,
                                value: bridge_core::types::MetricValue::Gauge(0.0),
                                labels: std::collections::HashMap::new(),
                                timestamp: batch.timestamp,
                            })
                        }),
                        attributes: record.metadata.clone(),
                        tags: std::collections::HashMap::new(),
                        resource: None,
                        service: None,
                    })
                    }
                    _ => None,
                }
            }).collect(),
            metadata: batch.metadata,
        };
        
        match self.writer.write_batch(telemetry_batch).await {
            Ok(result) => {
                total_records += result.records_written as u64;
                debug!("Successfully exported {} records to Apache Hudi", result.records_written);
            }
            Err(e) => {
                let error = bridge_core::types::ExportError {
                    code: "hudi_write_error".to_string(),
                    message: format!("Failed to export batch: {}", e),
                    details: None,
                };
                errors.push(error);
                error!("Failed to export batch to Apache Hudi: {}", e);
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
            info!("Real Apache Hudi exporter successfully exported {} records in {:?}",
                  total_records, export_duration);
        } else {
            warn!("Real Apache Hudi exporter failed to export some records: {} errors",
                  errors.len());
        }

        Ok(ExportResult {
            timestamp: chrono::Utc::now(),
            status: if errors.is_empty() { 
                bridge_core::types::ExportStatus::Success 
            } else { 
                bridge_core::types::ExportStatus::Partial 
            },
            records_exported: total_records as usize,
            records_failed: errors.len(),
            duration_ms: export_duration.as_millis() as u64,
            metadata: std::collections::HashMap::new(),
            errors,
        })
    }

    fn name(&self) -> &str {
        "apache-hudi-exporter"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        // For now, just check if the writer is initialized
        Ok(self.writer.is_initialized())
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ExporterStats> {
        let stats = self.stats.read().unwrap();
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

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Real Apache Hudi exporter shutting down");

        // Mark exporter as not running
        {
            let mut running = self.running.write().unwrap();
            *running = false;
        }

        // Close the writer
        let _ = self.writer.close().await;

        info!("Real Apache Hudi exporter shutdown completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HudiConfig;

    #[tokio::test]
    async fn test_real_hudi_exporter_creation() {
        let config = HudiConfig::new(
            "s3://hudi-tables".to_string(),
            "test_table".to_string(),
        );

        let exporter = RealHudiExporter::new(config).await;
        assert_eq!(exporter.name(), "test_table");
        assert_eq!(exporter.version(), "0.1.0");
        assert!(!exporter.is_running());
    }

    #[tokio::test]
    async fn test_real_hudi_exporter_initialization() {
        let config = HudiConfig::new(
            "s3://hudi-tables".to_string(),
            "test_table".to_string(),
        );

        let mut exporter = RealHudiExporter::new(config).await;
        let result = exporter.initialize().await;
        assert!(result.is_ok());

        // Check that exporter is running after initialization
        assert!(exporter.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_real_hudi_exporter_export() {
        let config = HudiConfig::new(
            "s3://hudi-tables".to_string(),
            "test_table".to_string(),
        );

        let mut exporter = RealHudiExporter::new(config).await;
        exporter.initialize().await.unwrap();

        // Create a test batch
        let telemetry_batch = bridge_core::types::TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: "test".to_string(),
            size: 1,
            records: vec![
                bridge_core::types::TelemetryRecord {
                    id: uuid::Uuid::new_v4(),
                    timestamp: chrono::Utc::now(),
                    record_type: bridge_core::types::TelemetryType::Metric,
                    data: bridge_core::types::TelemetryData::Metric(bridge_core::types::MetricData {
                        name: "test_metric".to_string(),
                        description: None,
                        unit: Some("count".to_string()),
                        metric_type: bridge_core::types::MetricType::Gauge,
                        value: bridge_core::types::MetricValue::Gauge(42.0),
                        labels: std::collections::HashMap::new(),
                        timestamp: chrono::Utc::now(),
                    }),
                    attributes: std::collections::HashMap::new(),
                    tags: std::collections::HashMap::new(),
                    resource: None,
                    service: None,
                }
            ],
            metadata: std::collections::HashMap::new(),
        };

        // Convert TelemetryBatch to ProcessedBatch for testing
        let batch = bridge_core::types::ProcessedBatch {
            original_batch_id: telemetry_batch.id,
            timestamp: telemetry_batch.timestamp,
            status: bridge_core::types::ProcessingStatus::Success,
            records: telemetry_batch.records.iter().map(|record| {
                bridge_core::types::ProcessedRecord {
                    original_id: record.id,
                    status: bridge_core::types::ProcessingStatus::Success,
                    transformed_data: Some(record.data.clone()),
                    metadata: record.attributes.clone(),
                    errors: vec![],
                }
            }).collect(),
            metadata: telemetry_batch.metadata,
            errors: vec![],
        };
        
        let result = exporter.export(batch).await;
        assert!(result.is_ok());

        let export_result = result.unwrap();
        assert_eq!(export_result.records_exported, 1);
        assert_eq!(export_result.errors.len(), 0);
    }
}
