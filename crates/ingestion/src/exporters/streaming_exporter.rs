//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming exporter implementation
//!
//! This module provides a streaming exporter for streaming telemetry data
//! to various destinations in real-time.

use async_trait::async_trait;
use bridge_core::{
    traits::LakehouseExporter as BridgeTelemetryExporter, BridgeResult, ExportResult,
    ProcessedBatch, TelemetryBatch,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;
use reqwest::Client;

use super::{BaseExporter, ExporterConfig};

/// Streaming exporter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingExporterConfig {
    /// Exporter name
    pub name: String,

    /// Exporter version
    pub version: String,

    /// Destination URL
    pub destination_url: String,

    /// Stream buffer size
    pub buffer_size: usize,

    /// Stream timeout in seconds
    pub stream_timeout_secs: u64,

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

impl StreamingExporterConfig {
    /// Create new streaming exporter configuration
    pub fn new(destination_url: String, buffer_size: usize) -> Self {
        Self {
            name: "streaming".to_string(),
            version: "1.0.0".to_string(),
            destination_url,
            buffer_size,
            stream_timeout_secs: 30,
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
impl ExporterConfig for StreamingExporterConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.destination_url.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Streaming exporter destination URL cannot be empty",
            ));
        }

        if self.buffer_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Streaming exporter buffer size cannot be 0",
            ));
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Streaming exporter implementation
pub struct StreamingExporter {
    base: BaseExporter,
    config: StreamingExporterConfig,
    stream_buffer: Arc<RwLock<Vec<ProcessedBatch>>>,
    http_client: Client,
    stats: Arc<RwLock<StreamingExporterStats>>,
}

/// Streaming exporter statistics
#[derive(Debug, Clone)]
struct StreamingExporterStats {
    total_batches: u64,
    total_records: u64,
    batches_per_minute: u64,
    records_per_minute: u64,
    avg_export_time_ms: f64,
    error_count: u64,
    last_export_time: Option<DateTime<Utc>>,
    last_minute_batches: Vec<DateTime<Utc>>,
    last_minute_records: Vec<DateTime<Utc>>,
    export_times: Vec<f64>,
}

impl StreamingExporter {
    /// Create new streaming exporter
    pub async fn new(config: &dyn ExporterConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<StreamingExporterConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration("Invalid streaming exporter configuration")
            })?
            .clone();

        config.validate().await?;

        let base = BaseExporter::new(config.name.clone(), config.version.clone());

        Ok(Self {
            base,
            config,
            stream_buffer: Arc::new(RwLock::new(Vec::new())),
            http_client: Client::builder()
                .timeout(tokio::time::Duration::from_secs(5))
                .build()
                .unwrap_or_else(|_| Client::new()),
            stats: Arc::new(RwLock::new(StreamingExporterStats {
                total_batches: 0,
                total_records: 0,
                batches_per_minute: 0,
                records_per_minute: 0,
                avg_export_time_ms: 0.0,
                error_count: 0,
                last_export_time: None,
                last_minute_batches: Vec::new(),
                last_minute_records: Vec::new(),
                export_times: Vec::new(),
            })),
        })
    }

    /// Stream batch to destination
    async fn stream_batch(&self, batch: ProcessedBatch) -> BridgeResult<()> {
        let start_time = std::time::Instant::now();
        let url = self.config.destination_url.clone();

        let mut request_builder = self.http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json");

        if let Some(auth_token) = &self.config.auth_token {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", auth_token));
        }

        for (key, value) in &self.config.headers {
            request_builder = request_builder.header(key, value);
        }

        let request_builder = request_builder.json(&batch);

        let response = request_builder
            .send()
            .await
            .map_err(|e| bridge_core::BridgeError::export(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status().to_string();
            let body = response.text().await.unwrap_or_default();
            return Err(bridge_core::BridgeError::export(format!(
                "HTTP request failed with status {}: {}",
                status, body
            )));
        }

        let export_time = start_time.elapsed().as_millis() as f64;
        self.update_success_stats(batch.records.len(), export_time).await;

        info!(
            "Successfully streamed processed batch {} with {} records to {} in {:.2}ms",
            batch.original_batch_id,
            batch.records.len(),
            self.config.destination_url,
            export_time
        );

        Ok(())
    }

    /// Update statistics for successful export
    async fn update_success_stats(&self, records_count: usize, export_time_ms: f64) {
        let mut stats = self.stats.write().await;
        let now = Utc::now();

        stats.total_batches += 1;
        stats.total_records += records_count as u64;
        stats.last_export_time = Some(now);
        stats.export_times.push(export_time_ms);

        // Keep only last 100 export times for average calculation
        if stats.export_times.len() > 100 {
            stats.export_times.remove(0);
        }

        // Update average export time
        stats.avg_export_time_ms = stats.export_times.iter().sum::<f64>() / stats.export_times.len() as f64;

        // Update per-minute statistics
        stats.last_minute_batches.push(now);
        for _ in 0..records_count {
            stats.last_minute_records.push(now);
        }

        // Remove entries older than 1 minute
        let one_minute_ago = now - chrono::Duration::minutes(1);
        stats.last_minute_batches.retain(|&time| time > one_minute_ago);
        stats.last_minute_records.retain(|&time| time > one_minute_ago);

        stats.batches_per_minute = stats.last_minute_batches.len() as u64;
        stats.records_per_minute = stats.last_minute_records.len() as u64;
    }

    /// Update statistics for failed export
    async fn update_error_stats(&self) {
        let mut stats = self.stats.write().await;
        stats.error_count += 1;
    }

    /// Add batch to stream buffer
    async fn add_to_buffer(&self, batch: ProcessedBatch) -> BridgeResult<()> {
        let mut buffer = self.stream_buffer.write().await;

        buffer.push(batch);

        // If buffer is full, flush it
        if buffer.len() >= self.config.buffer_size {
            self.flush_buffer().await?;
        }

        Ok(())
    }

    /// Flush stream buffer
    async fn flush_buffer(&self) -> BridgeResult<()> {
        let mut buffer = self.stream_buffer.write().await;

        if buffer.is_empty() {
            return Ok(());
        }

        let batches = std::mem::take(&mut *buffer);
        drop(buffer);

        // Stream all batches in buffer
        for batch in batches {
            self.stream_batch(batch).await?;
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[async_trait]
impl BridgeTelemetryExporter for StreamingExporter {
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        let start_time = std::time::Instant::now();

        // Add batch to stream buffer
        let mut last_error = None;
        for attempt in 0..=self.config.retry_attempts {
            match self.add_to_buffer(batch.clone()).await {
                Ok(_) => {
                    let export_time = start_time.elapsed();
                    let export_time_ms = export_time.as_millis() as f64;

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
                    self.update_error_stats().await;
                    if attempt < self.config.retry_attempts {
                        warn!(
                            "Stream export attempt {} failed, retrying in {}ms: {}",
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
                            "Stream export failed after all retry attempts",
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
        let url = self.config.destination_url.clone();

        let mut request_builder = self.http_client
            .head(&url)
            .timeout(tokio::time::Duration::from_secs(self.config.stream_timeout_secs));

        if let Some(auth_token) = &self.config.auth_token {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", auth_token));
        }

        for (key, value) in &self.config.headers {
            request_builder = request_builder.header(key, value);
        }

        match request_builder.send().await {
            Ok(response) => {
                let is_healthy = response.status().is_success() || response.status().as_u16() == 405; // 405 Method Not Allowed is acceptable for HEAD
                if is_healthy {
                    info!("Streaming exporter health check passed for {}", url);
                } else {
                    warn!("Streaming exporter health check failed for {} with status {}", url, response.status());
                }
                Ok(is_healthy)
            }
            Err(e) => {
                error!("Streaming exporter health check failed for {}: {}", url, e);
                Ok(false)
            }
        }
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down streaming exporter");

        // Flush any remaining data in buffer
        self.flush_buffer().await?;

        info!("Streaming exporter shutdown completed");
        Ok(())
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ExporterStats> {
        let stats = self.stats.read().await;
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
}

impl StreamingExporter {
    /// Get streaming configuration
    pub fn get_config(&self) -> &StreamingExporterConfig {
        &self.config
    }

    /// Force flush buffer
    pub async fn force_flush(&self) -> BridgeResult<()> {
        self.flush_buffer().await
    }

    /// Get buffer size
    pub async fn get_buffer_size(&self) -> usize {
        let buffer = self.stream_buffer.read().await;
        buffer.len()
    }

    /// Get detailed statistics
    pub async fn get_detailed_stats(&self) -> StreamingExporterStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Check if exporter is healthy
    pub async fn is_healthy(&self) -> bool {
        match self.health_check().await {
            Ok(healthy) => healthy,
            Err(_) => false,
        }
    }

    /// Get destination URL
    pub fn get_destination_url(&self) -> &str {
        &self.config.destination_url
    }

    /// Update configuration (for dynamic configuration changes)
    pub async fn update_config(&mut self, new_config: StreamingExporterConfig) -> BridgeResult<()> {
        new_config.validate().await?;
        self.config = new_config;
        info!("Streaming exporter configuration updated");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_core::types::{ProcessedRecord, ProcessingStatus, ProcessingError};

    fn create_test_batch() -> ProcessedBatch {
        ProcessedBatch {
            original_batch_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            status: ProcessingStatus::Success,
            records: vec![
                ProcessedRecord {
                    original_id: Uuid::new_v4(),
                    status: ProcessingStatus::Success,
                    transformed_data: None,
                    metadata: HashMap::from([
                        ("test_key".to_string(), "test_value".to_string()),
                    ]),
                    errors: vec![],
                },
            ],
            metadata: HashMap::from([
                ("source".to_string(), "test".to_string()),
            ]),
            errors: vec![],
        }
    }

    #[tokio::test]
    async fn test_streaming_exporter_creation() {
        let config = StreamingExporterConfig::new("http://localhost:8080/test".to_string(), 100);
        let exporter = StreamingExporter::new(&config).await;
        assert!(exporter.is_ok());
    }

    #[tokio::test]
    async fn test_streaming_exporter_config_validation() {
        // Test empty URL
        let config = StreamingExporterConfig::new("".to_string(), 100);
        let exporter = StreamingExporter::new(&config).await;
        assert!(exporter.is_err());

        // Test zero buffer size
        let config = StreamingExporterConfig::new("http://localhost:8080/test".to_string(), 0);
        let exporter = StreamingExporter::new(&config).await;
        assert!(exporter.is_err());
    }

    #[tokio::test]
    async fn test_streaming_exporter_buffer_operations() {
        let config = StreamingExporterConfig::new("http://localhost:8080/test".to_string(), 2);
        let exporter = StreamingExporter::new(&config).await.unwrap();

        // Test buffer size
        assert_eq!(exporter.get_buffer_size().await, 0);

        // Test force flush on empty buffer
        assert!(exporter.force_flush().await.is_ok());
    }

    #[tokio::test]
    async fn test_streaming_exporter_stats() {
        let config = StreamingExporterConfig::new("http://localhost:8080/test".to_string(), 100);
        let exporter = StreamingExporter::new(&config).await.unwrap();

        // Test initial stats
        let stats = exporter.get_stats().await.unwrap();
        assert_eq!(stats.total_batches, 0);
        assert_eq!(stats.total_records, 0);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.avg_export_time_ms, 0.0);

        // Test detailed stats
        let detailed_stats = exporter.get_detailed_stats().await;
        assert_eq!(detailed_stats.total_batches, 0);
        assert_eq!(detailed_stats.total_records, 0);
    }

    #[tokio::test]
    async fn test_streaming_exporter_utility_methods() {
        let config = StreamingExporterConfig::new("http://localhost:8080/test".to_string(), 100);
        let exporter = StreamingExporter::new(&config).await.unwrap();

        // Test destination URL
        assert_eq!(exporter.get_destination_url(), "http://localhost:8080/test");

        // Test configuration access
        let exporter_config = exporter.get_config();
        assert_eq!(exporter_config.destination_url, "http://localhost:8080/test");
        assert_eq!(exporter_config.buffer_size, 100);
    }

    #[tokio::test]
    async fn test_streaming_exporter_health_check() {
        let config = StreamingExporterConfig::new("http://localhost:8080/test".to_string(), 100);
        let exporter = StreamingExporter::new(&config).await.unwrap();

        // Test health check (will likely fail since localhost:8080 is not running)
        let health = exporter.health_check().await;
        assert!(health.is_ok()); // Should not panic even if endpoint is unreachable
    }

    #[tokio::test]
    async fn test_streaming_exporter_shutdown() {
        let config = StreamingExporterConfig::new("http://localhost:8080/test".to_string(), 100);
        let exporter = StreamingExporter::new(&config).await.unwrap();

        // Test shutdown
        assert!(exporter.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn test_streaming_exporter_export_with_retry() {
        let config = StreamingExporterConfig::new("http://localhost:8080/test".to_string(), 1); // Small buffer to force flush
        let exporter = StreamingExporter::new(&config).await.unwrap();

        let batch = create_test_batch();

        // Test export - this should succeed initially (buffer operation)
        let result = exporter.export(batch).await;
        assert!(result.is_ok());

        let export_result = result.unwrap();
        // The export should succeed initially (buffer operation)
        assert_eq!(export_result.status, bridge_core::types::ExportStatus::Success);
        assert_eq!(export_result.records_exported, 1);
        assert_eq!(export_result.records_failed, 0);
    }
}
