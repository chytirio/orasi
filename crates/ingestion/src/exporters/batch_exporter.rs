//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Batch exporter implementation
//!
//! This module provides a batch exporter for exporting telemetry data
//! in batches to various destinations.

use async_trait::async_trait;
use bridge_core::{
    traits::LakehouseExporter as BridgeTelemetryExporter, BridgeResult, ExportResult,
    ProcessedBatch,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use super::{BaseExporter, ExporterConfig};

/// Batch exporter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchExporterConfig {
    /// Exporter name
    pub name: String,

    /// Exporter version
    pub version: String,

    /// Destination URL
    pub destination_url: String,

    /// Batch size for export
    pub batch_size: usize,

    /// Export timeout in seconds
    pub export_timeout_secs: u64,

    /// Retry attempts
    pub retry_attempts: u32,

    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,

    /// Enable compression
    pub enable_compression: bool,

    /// Authentication token (optional)
    pub auth_token: Option<String>,

    /// Additional headers
    pub headers: HashMap<String, String>,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl BatchExporterConfig {
    /// Create new batch exporter configuration
    pub fn new(destination_url: String, batch_size: usize) -> Self {
        Self {
            name: "batch".to_string(),
            version: "1.0.0".to_string(),
            destination_url,
            batch_size,
            export_timeout_secs: 30,
            retry_attempts: 3,
            retry_delay_ms: 1000,
            enable_compression: true,
            auth_token: None,
            headers: HashMap::new(),
            additional_config: HashMap::new(),
        }
    }
}

#[async_trait]
impl ExporterConfig for BatchExporterConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.destination_url.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Batch exporter destination URL cannot be empty",
            ));
        }

        if self.batch_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Batch exporter batch size cannot be 0",
            ));
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Batch exporter implementation
pub struct BatchExporter {
    base: BaseExporter,
    config: BatchExporterConfig,
    stats: Arc<RwLock<bridge_core::traits::ExporterStats>>,
}

impl BatchExporter {
    /// Create new batch exporter
    pub async fn new(config: &dyn ExporterConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<BatchExporterConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration("Invalid batch exporter configuration")
            })?
            .clone();

        config.validate().await?;

        let base = BaseExporter::new(config.name.clone(), config.version.clone());

        let stats = Arc::new(RwLock::new(bridge_core::traits::ExporterStats {
            total_batches: 0,
            total_records: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            avg_export_time_ms: 0.0,
            error_count: 0,
            last_export_time: None,
        }));

        Ok(Self {
            base,
            config,
            stats,
        })
    }

    /// Update statistics after export operation
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

        // Calculate per-minute rates (simplified calculation)
        // In a production implementation, you might want to use a sliding window
        if let Some(last_export) = stats.last_export_time {
            let now = Utc::now();
            let time_diff = now.signed_duration_since(last_export).num_minutes();
            if time_diff > 0 {
                stats.batches_per_minute = stats.total_batches / time_diff as u64;
                stats.records_per_minute = stats.total_records / time_diff as u64;
            }
        }
    }

    /// Export batch to destination
    async fn export_batch(&self, batch: ProcessedBatch) -> BridgeResult<()> {
        // Send the batch to the configured destination via HTTP POST

        info!(
            "Exporting processed batch {} with {} records to {}",
            batch.original_batch_id,
            batch.records.len(),
            self.config.destination_url
        );

        // Create HTTP client
        let client = reqwest::Client::new();

        // Serialize batch to JSON
        let json_data = serde_json::to_vec(&batch).map_err(|e| {
            bridge_core::BridgeError::serialization(format!(
                "Failed to serialize batch to JSON: {}",
                e
            ))
        })?;

        // Send HTTP POST request
        let response = client
            .post(&self.config.destination_url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "OpenTelemetry-Bridge/1.0")
            .body(json_data)
            .send()
            .await
            .map_err(|e| {
                bridge_core::BridgeError::network(format!(
                    "Failed to send batch to destination: {}",
                    e
                ))
            })?;

        // Check response status
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(bridge_core::BridgeError::export(format!(
                "Export failed with status {}: {}",
                status, error_body
            )));
        }

        info!(
            "Successfully exported batch {} to {}",
            batch.original_batch_id, self.config.destination_url
        );

        Ok(())
    }
}

#[async_trait]
impl BridgeTelemetryExporter for BatchExporter {
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        let start_time = std::time::Instant::now();

        // Export batch with retry logic
        let mut last_error = None;
        for attempt in 0..=self.config.retry_attempts {
            match self.export_batch(batch.clone()).await {
                Ok(_) => {
                    let export_time = start_time.elapsed();
                    let export_time_ms = export_time.as_millis() as f64;

                    self.update_stats(batch.records.len(), export_time, true)
                        .await;

                    return Ok(ExportResult {
                        timestamp: Utc::now(),
                        status: bridge_core::types::ExportStatus::Success,
                        records_exported: batch.records.len(),
                        records_failed: 0,
                        duration_ms: export_time_ms as u64,
                        metadata: HashMap::new(),
                        errors: vec![],
                    });
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    last_error = Some(e);
                    if attempt < self.config.retry_attempts {
                        warn!(
                            "Export attempt {} failed, retrying in {}ms: {}",
                            attempt + 1,
                            self.config.retry_delay_ms,
                            error_msg
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            self.config.retry_delay_ms,
                        ))
                        .await;
                    }
                }
            }
        }

        // All retry attempts failed
        let export_time = start_time.elapsed();
        let export_time_ms = export_time.as_millis() as f64;

        self.update_stats(batch.records.len(), export_time, false)
            .await;

        Ok(ExportResult {
            timestamp: Utc::now(),
            status: bridge_core::types::ExportStatus::Failed,
            records_exported: 0,
            records_failed: batch.records.len(),
            duration_ms: export_time_ms as u64,
            metadata: HashMap::new(),
            errors: vec![bridge_core::types::ExportError {
                code: "EXPORT_FAILED".to_string(),
                message: last_error
                    .unwrap_or_else(|| {
                        bridge_core::BridgeError::export(
                            "Batch export failed after all retry attempts",
                        )
                    })
                    .to_string(),
                details: None,
            }],
        })
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn version(&self) -> &str {
        &self.base.version
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        // Check if the destination is reachable via HTTP HEAD request

        let client = reqwest::Client::new();

        // Try to send a HEAD request to check connectivity
        match client
            .head(&self.config.destination_url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => {
                let is_healthy =
                    response.status().is_success() || response.status().as_u16() == 405; // 405 Method Not Allowed is also OK
                if !is_healthy {
                    warn!(
                        "Health check failed for {}: status {}",
                        self.config.destination_url,
                        response.status()
                    );
                }
                Ok(is_healthy)
            }
            Err(e) => {
                error!(
                    "Health check failed for {}: {}",
                    self.config.destination_url, e
                );
                Ok(false)
            }
        }
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Batch exporter shutdown completed");
        Ok(())
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ExporterStats> {
        // Return current exporter statistics
        Ok(self.stats.read().await.clone())
    }
}

impl BatchExporter {
    /// Get batch configuration
    pub fn get_config(&self) -> &BatchExporterConfig {
        &self.config
    }
}
