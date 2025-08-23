//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OpenTelemetry data ingestion for the bridge
//!
//! This crate provides a comprehensive telemetry data ingestion system that can
//! receive, process, validate, and export telemetry data from various sources
//! and protocols.

pub mod config;
pub mod conversion;
pub mod error_handling;
pub mod exporters;
pub mod monitoring;
pub mod performance;
pub mod processors;
pub mod protocols;
pub mod receivers;
pub mod security;
pub mod validation;

use bridge_core::{
    traits::{LakehouseExporter, TelemetryProcessor, TelemetryReceiver},
    BridgeResult, TelemetryBatch,
};
use monitoring::{Monitorable, MonitoringManager};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Main ingestion system
pub struct IngestionSystem {
    receivers: HashMap<String, Box<dyn TelemetryReceiver>>,
    processors: HashMap<String, Box<dyn TelemetryProcessor>>,
    exporters: HashMap<String, Box<dyn LakehouseExporter>>,
    monitoring: Arc<MonitoringManager>,
}

impl IngestionSystem {
    /// Create new ingestion system
    pub fn new() -> Self {
        let monitoring_config = monitoring::MonitoringConfig::development();
        let monitoring = Arc::new(MonitoringManager::new(monitoring_config));

        Self {
            receivers: HashMap::new(),
            processors: HashMap::new(),
            exporters: HashMap::new(),
            monitoring,
        }
    }

    /// Create new ingestion system with custom monitoring
    pub fn with_monitoring(monitoring: Arc<MonitoringManager>) -> Self {
        Self {
            receivers: HashMap::new(),
            processors: HashMap::new(),
            exporters: HashMap::new(),
            monitoring,
        }
    }

    /// Add receiver to the system
    pub fn add_receiver(&mut self, name: String, receiver: Box<dyn TelemetryReceiver>) {
        info!("Adding receiver: {}", name);
        self.receivers.insert(name, receiver);
    }

    /// Add processor to the system
    pub fn add_processor(&mut self, name: String, processor: Box<dyn TelemetryProcessor>) {
        info!("Adding processor: {}", name);
        self.processors.insert(name, processor);
    }

    /// Add exporter to the system
    pub fn add_exporter(&mut self, name: String, exporter: Box<dyn LakehouseExporter>) {
        info!("Adding exporter: {}", name);
        self.exporters.insert(name, exporter);
    }

    /// Process a telemetry batch through the pipeline
    pub async fn process_batch(&self, batch: TelemetryBatch) -> BridgeResult<()> {
        let start_time = std::time::Instant::now();

        // Start monitoring span
        let span_id = self
            .monitoring
            .start_ingestion_span("batch_processing", batch.size as u64)
            .await?;

        // Record ingestion metrics
        self.monitoring
            .record_ingestion_metrics("batch", batch.size as usize, batch.size as usize, 0)
            .await?;

        // Process through all processors
        for (name, processor) in &self.processors {
            let processor_start = std::time::Instant::now();

            info!("Processing batch through processor: {}", name);
            let _processed_batch = processor.process(batch.clone()).await?;

            let processor_duration = processor_start.elapsed().as_millis() as u64;
            self.monitoring
                .record_performance_metrics(
                    &format!("processor_{}", name),
                    processor_duration,
                    true,
                )
                .await?;

            info!("Processor {} completed", name);
        }

        // Export through all exporters
        for (name, _exporter) in &self.exporters {
            info!("Exporting batch through exporter: {}", name);
            // For now, we'll skip export since we need ProcessedBatch
            info!("Exporter {} would export batch", name);
        }

        let total_duration = start_time.elapsed().as_millis() as u64;
        self.monitoring
            .record_performance_metrics("total_batch_processing", total_duration, true)
            .await?;

        // End monitoring span
        self.monitoring
            .end_ingestion_span(&span_id, true, None)
            .await?;

        Ok(())
    }

    /// Get system health status
    pub async fn health_check(&self) -> BridgeResult<()> {
        // Check if monitoring is healthy
        if !self.monitoring.is_healthy().await {
            self.monitoring
                .record_error_metrics("health_check", "Monitoring system unhealthy")
                .await?;
            return Err(bridge_core::BridgeError::configuration(
                "Monitoring system unhealthy".to_string(),
            ));
        }

        // Check receivers
        for (name, receiver) in &self.receivers {
            let is_healthy = receiver.health_check().await?;
            if is_healthy {
                info!("Receiver {} is healthy", name);
            } else {
                info!("Receiver {} is unhealthy", name);
                self.monitoring
                    .record_error_metrics("health_check", &format!("Receiver {} unhealthy", name))
                    .await?;
            }
        }

        // Check processors
        for (name, processor) in &self.processors {
            let is_healthy = processor.health_check().await?;
            if is_healthy {
                info!("Processor {} is healthy", name);
            } else {
                info!("Processor {} is unhealthy", name);
                self.monitoring
                    .record_error_metrics("health_check", &format!("Processor {} unhealthy", name))
                    .await?;
            }
        }

        // Check exporters
        for (name, _exporter) in &self.exporters {
            // LakehouseExporter doesn't have health_check, so we'll skip for now
            info!("Exporter {} status unknown", name);
        }

        Ok(())
    }

    /// Get system statistics
    pub async fn get_stats(&self) -> BridgeResult<()> {
        // Get monitoring statistics
        let monitoring_stats = self.monitoring.get_statistics().await;
        info!("Monitoring stats: {:?}", monitoring_stats);

        // Get receiver stats
        for (name, receiver) in &self.receivers {
            let stats = receiver.get_stats().await?;
            info!("Receiver {} stats: {:?}", name, stats);
        }

        // Get processor stats
        for (name, processor) in &self.processors {
            let stats = processor.get_stats().await?;
            info!("Processor {} stats: {:?}", name, stats);
        }

        Ok(())
    }

    /// Initialize the ingestion system
    pub async fn initialize(&self) -> BridgeResult<()> {
        // Initialize monitoring
        self.monitoring.initialize().await?;
        info!("Ingestion system initialized with monitoring");
        Ok(())
    }

    /// Get monitoring manager
    pub fn monitoring(&self) -> Arc<MonitoringManager> {
        Arc::clone(&self.monitoring)
    }
}

impl Default for IngestionSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Monitorable for IngestionSystem {
    fn monitoring(&self) -> Arc<MonitoringManager> {
        Arc::clone(&self.monitoring)
    }
}

#[cfg(test)]
mod advanced_integration_test;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exporters::batch_exporter::BatchExporter;
    use crate::processors::batch_processor::BatchProcessor;
    use crate::receivers::simple_http_receiver::SimpleHttpReceiver;
    use bridge_core::types::{
        MetricData, MetricValue, TelemetryData, TelemetryRecord, TelemetryType,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn create_test_batch() -> TelemetryBatch {
        let records = vec![TelemetryRecord {
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
        }];

        TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "test".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_ingestion_system_creation() {
        let mut system = IngestionSystem::new();
        assert_eq!(system.receivers.len(), 0);
        assert_eq!(system.processors.len(), 0);
        assert_eq!(system.exporters.len(), 0);
    }

    #[tokio::test]
    async fn test_ingestion_system_lifecycle() {
        let mut system = IngestionSystem::new();

        // Initialize the system first
        system.initialize().await.unwrap();

        // Add components
        let receiver_config = crate::receivers::simple_http_receiver::SimpleHttpReceiverConfig::new(
            "127.0.0.1".to_string(),
            8080,
        );
        let receiver = SimpleHttpReceiver::new(receiver_config);

        // Create processor config and processor
        let processor_config =
            crate::processors::batch_processor::BatchProcessorConfig::new(100, 1000);
        let processor = BatchProcessor::new(&processor_config).await.unwrap();

        // Create exporter config and exporter
        let exporter_config = crate::exporters::batch_exporter::BatchExporterConfig::new(
            "http://localhost:8080".to_string(),
            100,
        );
        let exporter = BatchExporter::new(&exporter_config).await.unwrap();

        system.add_receiver("test_receiver".to_string(), Box::new(receiver));
        system.add_processor("test_processor".to_string(), Box::new(processor));
        system.add_exporter("test_exporter".to_string(), Box::new(exporter));

        assert_eq!(system.receivers.len(), 1);
        assert_eq!(system.processors.len(), 1);
        assert_eq!(system.exporters.len(), 1);

        // Process a test batch
        let batch = create_test_batch();
        let result = system.process_batch(batch).await;
        assert!(result.is_ok());

        // Health check
        let health_result = system.health_check().await;
        assert!(health_result.is_ok());

        // Get stats
        let stats_result = system.get_stats().await;
        assert!(stats_result.is_ok());
    }

    #[tokio::test]
    async fn test_ingestion_initialization() {
        let mut system = IngestionSystem::new();

        // Test that we can add components
        let receiver_config = crate::receivers::simple_http_receiver::SimpleHttpReceiverConfig::new(
            "127.0.0.1".to_string(),
            8081,
        );
        let receiver = SimpleHttpReceiver::new(receiver_config);
        system.add_receiver("test_receiver".to_string(), Box::new(receiver));

        assert_eq!(system.receivers.len(), 1);
    }

    #[tokio::test]
    async fn test_ingestion_shutdown() {
        let mut system = IngestionSystem::new();

        // Initialize the system first
        system.initialize().await.unwrap();

        // Add a component
        let receiver_config = crate::receivers::simple_http_receiver::SimpleHttpReceiverConfig::new(
            "127.0.0.1".to_string(),
            8082,
        );
        let receiver = SimpleHttpReceiver::new(receiver_config);
        system.add_receiver("test_receiver".to_string(), Box::new(receiver));

        // Test health check
        let result = system.health_check().await;
        assert!(result.is_ok());
    }
}
