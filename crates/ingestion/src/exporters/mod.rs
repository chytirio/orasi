//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry exporters for OpenTelemetry data ingestion
//!
//! This module provides exporter implementations for exporting processed
//! telemetry data to various destinations.

pub mod batch_exporter;
pub mod streaming_exporter;

// Re-export exporter implementations
pub use batch_exporter::BatchExporter;
pub use streaming_exporter::StreamingExporter;

use async_trait::async_trait;
use bridge_core::{
    traits::LakehouseExporter as BridgeTelemetryExporter, BridgeResult, ExportResult,
    ProcessedBatch, TelemetryBatch, types::ProcessedRecord,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Exporter configuration trait
#[async_trait]
pub trait ExporterConfig: Send + Sync {
    /// Get exporter name
    fn name(&self) -> &str;

    /// Get exporter version
    fn version(&self) -> &str;

    /// Validate configuration
    async fn validate(&self) -> BridgeResult<()>;

    /// Get configuration as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Base exporter implementation
pub struct BaseExporter {
    name: String,
    version: String,
    is_running: Arc<RwLock<bool>>,
}

impl BaseExporter {
    /// Create new base exporter
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            is_running: Arc::new(RwLock::new(false)),
        }
    }
}

/// Exporter factory for creating exporters
pub struct ExporterFactory;

impl ExporterFactory {
    /// Create an exporter based on configuration
    pub async fn create_exporter(
        config: &dyn ExporterConfig,
    ) -> BridgeResult<Box<dyn BridgeTelemetryExporter>> {
        match config.name() {
            "batch" => {
                let exporter = BatchExporter::new(config).await?;
                Ok(Box::new(exporter))
            }
            "streaming" => {
                let exporter = StreamingExporter::new(config).await?;
                Ok(Box::new(exporter))
            }
            _ => Err(bridge_core::BridgeError::configuration(format!(
                "Unsupported exporter: {}",
                config.name()
            ))),
        }
    }
}

/// Exporter pipeline for chaining multiple exporters
pub struct ExporterPipeline {
    exporters: Vec<Box<dyn BridgeTelemetryExporter>>,
    is_running: Arc<RwLock<bool>>,
}

impl ExporterPipeline {
    /// Create new exporter pipeline
    pub fn new() -> Self {
        Self {
            exporters: Vec::new(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Add exporter to pipeline
    pub fn add_exporter(&mut self, exporter: Box<dyn BridgeTelemetryExporter>) {
        self.exporters.push(exporter);
    }

    /// Export batch through pipeline
    pub async fn export(&self, batch: TelemetryBatch) -> BridgeResult<Vec<ExportResult>> {
        let mut results = Vec::new();

        for exporter in &self.exporters {
            let start_time = std::time::Instant::now();

            // Convert TelemetryBatch to ProcessedBatch before exporting
            let processed_batch = ProcessedBatch {
                original_batch_id: batch.id,
                timestamp: Utc::now(),
                status: bridge_core::types::ProcessingStatus::Success,
                records: batch
                    .records
                    .iter()
                    .map(|r| ProcessedRecord {
                        original_id: r.id,
                        status: bridge_core::types::ProcessingStatus::Success,
                        transformed_data: Some(r.data.clone()),
                        metadata: r.attributes.clone(),
                        errors: vec![],
                    })
                    .collect(),
                metadata: batch.metadata.clone(),
                errors: vec![],
            };

            match exporter.export(processed_batch).await {
                Ok(export_result) => {
                    results.push(export_result);
                }
                Err(e) => {
                    error!("Exporter {} failed: {}", exporter.name(), e);
                    return Err(e);
                }
            }

            let export_time = start_time.elapsed();
            info!(
                "Exporter {} completed in {:?}",
                exporter.name(),
                export_time
            );
        }

        Ok(results)
    }

    /// Start all exporters
    pub async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting exporter pipeline");

        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        for exporter in &mut self.exporters {
            info!("Starting exporter: {}", exporter.name());
            if let Err(e) = exporter.health_check().await {
                error!("Failed to start exporter {}: {}", exporter.name(), e);
                return Err(e);
            }
        }

        info!("Exporter pipeline started");
        Ok(())
    }

    /// Stop all exporters
    pub async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping exporter pipeline");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        for exporter in &mut self.exporters {
            info!("Stopping exporter: {}", exporter.name());
            if let Err(e) = exporter.shutdown().await {
                error!("Failed to stop exporter {}: {}", exporter.name(), e);
                return Err(e);
            }
        }

        info!("Exporter pipeline stopped");
        Ok(())
    }

    /// Check if pipeline is running
    pub async fn is_running(&self) -> bool {
        let is_running = self.is_running.read().await;
        *is_running
    }
}
