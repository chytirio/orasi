//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Delta Lake exporter implementation
//! 
//! This module provides a real Delta Lake exporter that uses the actual
//! Delta Lake writer to write telemetry data to Delta Lake tables.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use async_trait::async_trait;
use tracing::{info, warn};
use chrono::Utc;

use bridge_core::traits::{LakehouseExporter, ExporterStats};
use bridge_core::types::{ProcessedBatch, ExportResult, ExportStatus};
use bridge_core::error::BridgeResult;

use crate::config::DeltaLakeConfig;
use crate::error::DeltaLakeResult;
use crate::writer::RealDeltaLakeWriter;

/// Real Delta Lake exporter implementation
pub struct RealDeltaLakeExporter {
    /// Delta Lake configuration
    config: DeltaLakeConfig,
    
    /// Delta Lake writer
    writer: Arc<RealDeltaLakeWriter>,
    
    /// Exporter state
    is_running: Arc<RwLock<bool>>,
    
    /// Exporter statistics
    stats: Arc<RwLock<ExporterStats>>,
}

impl RealDeltaLakeExporter {
    /// Create a new Delta Lake exporter
    pub fn new(config: DeltaLakeConfig) -> Self {
        let writer = RealDeltaLakeWriter::new(config.clone());
        
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
            config,
            writer: Arc::new(writer),
            is_running: Arc::new(RwLock::new(false)), // Start as not running
            stats: Arc::new(RwLock::new(stats)),
        }
    }

    /// Initialize the exporter
    pub async fn initialize(&mut self) -> DeltaLakeResult<()> {
        info!("Initializing Delta Lake exporter: {}", self.config.table_name);
        
        // Initialize the writer
        let mut writer = RealDeltaLakeWriter::new(self.config.clone());
        writer.initialize().await?;
        
        // Replace the writer with the initialized one
        self.writer = Arc::new(writer);
        
        // Mark as running
        {
            let mut is_running = self.is_running.write().await;
            *is_running = true;
        }
        
        info!("Delta Lake exporter initialized successfully: {}", self.config.table_name);
        Ok(())
    }

    async fn update_stats(&self, records: usize, duration: Duration, success: bool) {
        let mut stats = self.stats.write().await;
        
        stats.total_batches += 1;
        stats.total_records += records as u64;
        stats.last_export_time = Some(Utc::now());
        
        if success {
            let duration_ms = duration.as_millis() as f64;
            if stats.total_batches > 1 {
                stats.avg_export_time_ms = 
                    (stats.avg_export_time_ms * (stats.total_batches - 1) as f64 + duration_ms) 
                    / stats.total_batches as f64;
            } else {
                stats.avg_export_time_ms = duration_ms;
            }
        } else {
            stats.error_count += 1;
        }
    }

    async fn real_delta_lake_export(&self, batch: &ProcessedBatch) -> (bool, usize, Vec<String>) {
        let start_time = Instant::now();
        
        // Use the real Delta Lake writer to write the batch
        match self.writer.write(batch.clone()).await {
            Ok(_) => {
                let duration = start_time.elapsed();
                info!("Real Delta Lake export completed: {} records in {:?}", 
                      batch.records.len(), duration);
                (true, batch.records.len(), Vec::new())
            }
            Err(e) => {
                warn!("Real Delta Lake export failed: {}", e);
                (false, 0, vec![e.to_string()])
            }
        }
    }
}

#[async_trait]
impl LakehouseExporter for RealDeltaLakeExporter {
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        let start_time = Instant::now();
        
        // Check if exporter is running
        if !*self.is_running.read().await {
            return Err(bridge_core::error::BridgeError::export(
                "Delta Lake exporter is not running"
            ));
        }
        
        info!("Real Delta Lake exporter exporting batch with {} records", batch.records.len());
        
        // Real Delta Lake export operation
        let (success, records_written, errors) = self.real_delta_lake_export(&batch).await;
        
        let duration = start_time.elapsed();
        
        // Update statistics
        self.update_stats(batch.records.len(), duration, success).await;
        
        // Convert errors to ExportError format
        let export_errors: Vec<bridge_core::types::ExportError> = errors.iter().map(|e| bridge_core::types::ExportError {
            code: "DELTA_LAKE_ERROR".to_string(),
            message: e.clone(),
            details: None,
        }).collect();
        
        // Create export result
        let export_result = ExportResult {
            timestamp: Utc::now(),
            status: if success {
                ExportStatus::Success
            } else {
                ExportStatus::Failed
            },
            records_exported: records_written,
            records_failed: batch.records.len() - records_written,
            duration_ms: duration.as_millis() as u64,
            metadata: HashMap::new(),
            errors: export_errors,
        };
        
        if success {
            info!("Real Delta Lake exporter successfully exported {} records in {:?}", 
                  records_written, duration);
        } else {
            warn!("Real Delta Lake exporter failed to export some records: {} errors", 
                  errors.len());
        }
        
        Ok(export_result)
    }
    
    fn name(&self) -> &str {
        &self.config.table_name
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    async fn health_check(&self) -> BridgeResult<bool> {
        Ok(*self.is_running.read().await)
    }
    
    async fn get_stats(&self) -> BridgeResult<ExporterStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Real Delta Lake exporter shutting down");
        
        // Shutdown the writer
        self.writer.shutdown().await?;
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("Real Delta Lake exporter shutdown completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use uuid::Uuid;
    use bridge_core::types::{TelemetryRecord, TelemetryType, TelemetryData, MetricData, MetricValue, ProcessedRecord, ProcessingStatus};

    #[tokio::test]
    async fn test_real_delta_lake_exporter_creation() {
        let config = DeltaLakeConfig {
            table_name: "test_table".to_string(),
            table_path: "/tmp/test_delta_table".to_string(),
            ..Default::default()
        };
        
        let exporter = RealDeltaLakeExporter::new(config);
        assert_eq!(exporter.name(), "test_table");
        assert_eq!(exporter.version(), "1.0.0");
    }

    #[tokio::test]
    async fn test_real_delta_lake_exporter_initialization() {
        let temp_dir = tempdir().unwrap();
        let config = DeltaLakeConfig {
            table_name: "test_table".to_string(),
            table_path: temp_dir.path().join("test_delta_table").to_string_lossy().to_string(),
            ..Default::default()
        };
        
        let mut exporter = RealDeltaLakeExporter::new(config);
        let result = exporter.initialize().await;
        assert!(result.is_ok());
        
        // Check that exporter is running after initialization
        assert!(exporter.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_real_delta_lake_exporter_export() {
        let temp_dir = tempdir().unwrap();
        let config = DeltaLakeConfig {
            table_name: "test_table".to_string(),
            table_path: temp_dir.path().join("test_delta_table").to_string_lossy().to_string(),
            ..Default::default()
        };
        
        let mut exporter = RealDeltaLakeExporter::new(config);
        exporter.initialize().await.unwrap();
        
        // Create a test batch
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "test_metric".to_string(),
                description: Some("Test metric".to_string()),
                unit: Some("count".to_string()),
                metric_type: bridge_core::types::MetricType::Gauge,
                value: MetricValue::Gauge(42.0),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };
        
        let processed_record = ProcessedRecord {
            original_id: Uuid::new_v4(),
            status: ProcessingStatus::Success,
            transformed_data: Some(record.data.clone()),
            metadata: HashMap::new(),
            errors: Vec::new(),
        };
        
        let batch = ProcessedBatch {
            original_batch_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            records: vec![processed_record],
            metadata: HashMap::new(),
            status: ProcessingStatus::Success,
            errors: Vec::new(),
        };
        
        // Export the batch
        let result = exporter.export(batch).await.unwrap();
        assert_eq!(result.status, ExportStatus::Success);
        assert_eq!(result.records_exported, 1);
        assert_eq!(result.records_failed, 0);
    }
}
