//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Apache Kafka exporter implementation
//! 
//! This module provides a real Apache Kafka exporter that uses the actual
//! Kafka producer to export telemetry data to Kafka topics.

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use async_trait::async_trait;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bridge_core::traits::{LakehouseExporter, ExporterStats, LakehouseWriter};
use bridge_core::types::{ProcessedBatch, ExportResult, ExportError, ExportStatus, MetricsBatch, TracesBatch, LogsBatch, TelemetryData, ProcessedRecord};
use std::collections::HashMap;
use bridge_core::error::BridgeResult;

use crate::config::KafkaConfig;
use crate::error::{KafkaError, KafkaResult};
use crate::producer::KafkaProducer;

/// Real Apache Kafka exporter implementation
pub struct RealKafkaExporter {
    /// Kafka configuration
    config: KafkaConfig,
    /// Kafka producer instance
    producer: Arc<KafkaProducer>,
    /// Exporter state
    running: Arc<RwLock<bool>>,
    /// Exporter statistics
    stats: Arc<RwLock<ExporterStats>>,
}

impl RealKafkaExporter {
    /// Create a new Apache Kafka exporter
    pub async fn new(config: KafkaConfig) -> KafkaResult<Self> {
        let stats = ExporterStats {
            total_batches: 0,
            total_records: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            avg_export_time_ms: 0.0,
            error_count: 0,
            last_export_time: None,
        };

        let producer = KafkaProducer::new(config.clone()).await?;

        Ok(Self {
            producer: Arc::new(producer),
            config,
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
        })
    }

    /// Initialize the exporter
    pub async fn initialize(&mut self) -> KafkaResult<()> {
        info!("Initializing Apache Kafka exporter: {}", self.config.topic_name());
        
        // Mark exporter as running
        {
            let mut running = self.running.write().unwrap();
            *running = true;
        }
        
        info!("Apache Kafka exporter initialized successfully: {}", self.config.topic_name());
        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &KafkaConfig {
        &self.config
    }

    /// Check if the exporter is running
    pub fn is_running(&self) -> bool {
        *self.running.read().unwrap()
    }


}

#[async_trait]
impl LakehouseExporter for RealKafkaExporter {
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        // Check if exporter is running
        if !self.is_running() {
            return Err(bridge_core::error::BridgeError::export(
                "Apache Kafka exporter is not running"
            ));
        }

        let start_time = Instant::now();
        info!("Real Apache Kafka exporter exporting batch with {} records", batch.records.len());

        let mut total_records = 0u64;
        let mut errors = Vec::new();

        // Process each record in the batch
        for (index, record) in batch.records.iter().enumerate() {
            if let Some(transformed_data) = &record.transformed_data {
                match transformed_data {
                    TelemetryData::Metric(metric_data) => {
                        // Create a single-record metrics batch
                        let metrics_batch = MetricsBatch {
                            id: uuid::Uuid::new_v4(),
                            timestamp: chrono::Utc::now(),
                            metrics: vec![metric_data.clone()],
                            metadata: HashMap::new(),
                        };
                        
                        match self.producer.write_metrics(metrics_batch).await {
                            Ok(_) => {
                                total_records += 1;
                                debug!("Successfully exported metric record to Apache Kafka");
                            }
                            Err(e) => {
                                let error = ExportError {
                                    code: "kafka_produce_error".to_string(),
                                    message: format!("Failed to export metric: {}", e),
                                    details: None,
                                };
                                errors.push(error);
                                error!("Failed to export metric to Apache Kafka: {}", e);
                            }
                        }
                    }
                    TelemetryData::Trace(trace_data) => {
                        // Create a single-record traces batch
                        let traces_batch = TracesBatch {
                            id: uuid::Uuid::new_v4(),
                            timestamp: chrono::Utc::now(),
                            traces: vec![trace_data.clone()],
                            metadata: HashMap::new(),
                        };
                        
                        match self.producer.write_traces(traces_batch).await {
                            Ok(_) => {
                                total_records += 1;
                                debug!("Successfully exported trace record to Apache Kafka");
                            }
                            Err(e) => {
                                let error = ExportError {
                                    code: "kafka_produce_error".to_string(),
                                    message: format!("Failed to export trace: {}", e),
                                    details: None,
                                };
                                errors.push(error);
                                error!("Failed to export trace to Apache Kafka: {}", e);
                            }
                        }
                    }
                    TelemetryData::Log(log_data) => {
                        // Create a single-record logs batch
                        let logs_batch = LogsBatch {
                            id: uuid::Uuid::new_v4(),
                            timestamp: chrono::Utc::now(),
                            logs: vec![log_data.clone()],
                            metadata: HashMap::new(),
                        };
                        
                        match self.producer.write_logs(logs_batch).await {
                            Ok(_) => {
                                total_records += 1;
                                debug!("Successfully exported log record to Apache Kafka");
                            }
                            Err(e) => {
                                let error = ExportError {
                                    code: "kafka_produce_error".to_string(),
                                    message: format!("Failed to export log: {}", e),
                                    details: None,
                                };
                                errors.push(error);
                                error!("Failed to export log to Apache Kafka: {}", e);
                            }
                        }
                    }
                    TelemetryData::Event(_) => {
                        // Events are not currently supported in this exporter
                        debug!("Skipping event data - not supported in Kafka exporter");
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
            info!("Real Apache Kafka exporter successfully exported {} records in {:?}",
                  total_records, export_duration);
        } else {
            warn!("Real Apache Kafka exporter failed to export some records: {} errors",
                  errors.len());
        }

        Ok(ExportResult {
            timestamp: chrono::Utc::now(),
            status: if errors.is_empty() { ExportStatus::Success } else { ExportStatus::Partial },
            records_failed: errors.len(),
            duration_ms: export_duration.as_millis() as u64,
            metadata: HashMap::new(),
            errors,
            records_exported: total_records as usize,
        })
    }

    fn name(&self) -> &str {
        &self.config.topic.name
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        if !self.is_running() {
            return Ok(false);
        }

        // Check if producer is healthy
        let producer_healthy = self.producer.is_initialized();
        
        Ok(producer_healthy)
    }

    async fn get_stats(&self) -> BridgeResult<ExporterStats> {
        let stats = self.stats.read().unwrap();
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Real Apache Kafka exporter shutting down");

        // Mark exporter as not running
        {
            let mut running = self.running.write().unwrap();
            *running = false;
        }

        // Close the producer
        let _ = self.producer.close().await;

        info!("Real Apache Kafka exporter shutdown completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KafkaConfig;

    #[tokio::test]
    async fn test_real_kafka_exporter_creation() {
        let config = KafkaConfig::new(
            "localhost:9092".to_string(),
            "test_topic".to_string(),
            "test_group".to_string(),
        );

        let exporter = RealKafkaExporter::new(config).await.unwrap();
        assert_eq!(exporter.name(), "test_topic");
        assert_eq!(exporter.version(), env!("CARGO_PKG_VERSION"));
        assert!(!exporter.is_running());
    }

    #[tokio::test]
    async fn test_real_kafka_exporter_initialization() {
        let config = KafkaConfig::new(
            "localhost:9092".to_string(),
            "test_topic".to_string(),
            "test_group".to_string(),
        );

        let mut exporter = RealKafkaExporter::new(config).await.unwrap();
        let result = exporter.initialize().await;
        assert!(result.is_ok());

        // Check that exporter is running after initialization
        assert!(exporter.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_real_kafka_exporter_export() {
        let config = KafkaConfig::new(
            "localhost:9092".to_string(),
            "test_topic".to_string(),
            "test_group".to_string(),
        );

        let mut exporter = RealKafkaExporter::new(config).await.unwrap();
        exporter.initialize().await.unwrap();

        // Create a test batch
        let batch = ProcessedBatch {
            original_batch_id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            status: bridge_core::types::ProcessingStatus::Success,
            records: vec![
                ProcessedRecord {
                    original_id: uuid::Uuid::new_v4(),
                    status: bridge_core::types::ProcessingStatus::Success,
                    transformed_data: Some(TelemetryData::Metric(bridge_core::types::MetricData {
                        name: "test_metric".to_string(),
                        description: Some("Test metric".to_string()),
                        unit: Some("count".to_string()),
                        metric_type: bridge_core::types::MetricType::Counter,
                        value: bridge_core::types::MetricValue::Counter(42.0),
                        timestamp: chrono::Utc::now(),
                        labels: {
                            let mut map = HashMap::new();
                            map.insert("service".to_string(), "test".to_string());
                            map
                        },
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
        assert_eq!(export_result.status, ExportStatus::Success);
        assert_eq!(export_result.records_exported, 1);
        assert_eq!(export_result.errors.len(), 0);
    }
}
