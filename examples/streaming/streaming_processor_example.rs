//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating streaming processor with bridge-core pipeline
//!
//! This example shows how to create a streaming processor that implements
//! real-time processing capabilities for telemetry data.

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn};

use bridge_core::{
    pipeline::PipelineConfig,
    traits::{ExporterStats, StreamProcessor, StreamProcessorStats},
    types::{
        ExportResult, ExportStatus, MetricData, MetricValue, ProcessedBatch, ProcessingStatus,
        TelemetryData, TelemetryRecord, TelemetryType,
    },
    BridgeResult, LakehouseExporter, TelemetryIngestionPipeline, TelemetryReceiver,
};
use uuid::Uuid;

/// Simple mock receiver for streaming processor testing
pub struct StreamingMockReceiver {
    name: String,
    is_running: bool,
    record_counter: u64,
}

impl StreamingMockReceiver {
    pub fn new(name: String) -> Self {
        Self {
            name,
            is_running: false,
            record_counter: 0,
        }
    }
}

#[async_trait]
impl TelemetryReceiver for StreamingMockReceiver {
    async fn receive(&self) -> BridgeResult<bridge_core::types::TelemetryBatch> {
        // Create streaming telemetry records
        let mut records = Vec::new();

        // Generate multiple records to simulate streaming data
        for i in 0..5 {
            let record = TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name: format!("streaming_metric_{}", i),
                    description: Some(format!("Streaming metric {} for real-time processing", i)),
                    unit: Some("count".to_string()),
                    metric_type: bridge_core::types::MetricType::Gauge,
                    value: MetricValue::Gauge((self.record_counter + i as u64) as f64),
                    labels: HashMap::new(),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::new(),
                tags: HashMap::new(),
                resource: None,
                service: None,
            };
            records.push(record);
        }

        let batch = bridge_core::types::TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "streaming-test".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::new(),
        };

        info!(
            "Streaming mock receiver '{}' generated batch with {} records",
            self.name, batch.size
        );
        Ok(batch)
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        Ok(self.is_running)
    }

    async fn get_stats(&self) -> BridgeResult<bridge_core::traits::ReceiverStats> {
        Ok(bridge_core::traits::ReceiverStats {
            total_records: self.record_counter,
            records_per_minute: 5,
            total_bytes: 500,
            bytes_per_minute: 500,
            error_count: 0,
            last_receive_time: Some(Utc::now()),
            protocol_stats: None,
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Streaming mock receiver '{}' shutting down", self.name);
        Ok(())
    }
}

/// Real-time streaming processor implementation
pub struct RealTimeStreamingProcessor {
    name: String,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<StreamProcessorStats>>,
    processing_window_ms: u64,
    last_process_time: Arc<RwLock<Option<Instant>>>,
}

impl RealTimeStreamingProcessor {
    pub fn new(name: String) -> Self {
        let stats = StreamProcessorStats {
            total_records: 0,
            records_per_minute: 0,
            avg_processing_time_ms: 0.0,
            error_count: 0,
            last_process_time: None,
        };

        Self {
            name,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
            processing_window_ms: 1000, // 1 second processing window
            last_process_time: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn start(&mut self) {
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        info!("Real-time streaming processor '{}' started", self.name);
    }

    pub async fn stop(&mut self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        info!("Real-time streaming processor '{}' stopped", self.name);
    }

    async fn update_stats(&self, records: usize, duration: Duration, success: bool) {
        let mut stats = self.stats.write().await;

        stats.total_records += records as u64;
        stats.last_process_time = Some(Utc::now());

        if success {
            let duration_ms = duration.as_millis() as f64;
            if stats.total_records > records as u64 {
                stats.avg_processing_time_ms = (stats.avg_processing_time_ms
                    * (stats.total_records - records as u64) as f64
                    + duration_ms)
                    / stats.total_records as f64;
            } else {
                stats.avg_processing_time_ms = duration_ms;
            }
        } else {
            stats.error_count += 1;
        }
    }

    async fn real_time_process(&self, batch: &ProcessedBatch) -> ProcessedBatch {
        let start_time = Instant::now();

        info!(
            "Real-time processing batch with {} records",
            batch.records.len()
        );

        // Simulate real-time processing operations:
        // 1. Filter out records older than processing window
        // 2. Apply real-time transformations
        // 3. Calculate real-time aggregations
        // 4. Update processing statistics

        let mut processed_records = Vec::new();
        let current_time = Utc::now();

        for record in &batch.records {
            // Filter records within processing window
            if let Some(timestamp) = record
                .transformed_data
                .as_ref()
                .and_then(|_| Some(current_time))
            {
                let age_ms = (current_time - timestamp).num_milliseconds() as u64;

                if age_ms <= self.processing_window_ms {
                    // Apply real-time transformation
                    let mut processed_record = record.clone();

                    // Add real-time processing metadata
                    processed_record
                        .metadata
                        .insert("real_time_processed".to_string(), "true".to_string());
                    processed_record.metadata.insert(
                        "processing_window_ms".to_string(),
                        self.processing_window_ms.to_string(),
                    );
                    processed_record.metadata.insert(
                        "processing_timestamp".to_string(),
                        current_time.to_rfc3339(),
                    );

                    processed_records.push(processed_record);
                }
            }
        }

        // Update last process time
        {
            let mut last_time = self.last_process_time.write().await;
            *last_time = Some(start_time);
        }

        let duration = start_time.elapsed();
        self.update_stats(processed_records.len(), duration, true)
            .await;

        info!(
            "Real-time processing completed: {} records processed in {:?}",
            processed_records.len(),
            duration
        );

        ProcessedBatch {
            original_batch_id: batch.original_batch_id,
            timestamp: current_time,
            records: processed_records,
            metadata: {
                let mut metadata = batch.metadata.clone();
                metadata.insert("real_time_processor".to_string(), self.name.clone());
                metadata.insert(
                    "processing_duration_ms".to_string(),
                    duration.as_millis().to_string(),
                );
                metadata
            },
            status: ProcessingStatus::Success,
            errors: Vec::new(),
        }
    }
}

#[async_trait]
impl StreamProcessor for RealTimeStreamingProcessor {
    async fn process_stream(
        &self,
        input: bridge_core::traits::DataStream,
    ) -> BridgeResult<bridge_core::traits::DataStream> {
        let start_time = Instant::now();

        // Check if processor is running
        if !*self.is_running.read().await {
            return Err(bridge_core::error::BridgeError::internal(
                "Real-time streaming processor is not running",
            ));
        }

        info!(
            "Real-time streaming processor '{}' processing stream: {}",
            self.name, input.stream_id
        );

        // Simulate real-time stream processing
        // In a real implementation, this would:
        // 1. Parse the input stream data
        // 2. Apply real-time processing logic
        // 3. Transform the data
        // 4. Return processed stream

        let duration = start_time.elapsed();
        self.update_stats(input.data.len(), duration, true).await;

        // Create processed output stream
        let output_stream = bridge_core::traits::DataStream {
            stream_id: format!("processed_{}", input.stream_id),
            data: input.data, // In real implementation, this would be transformed data
            metadata: {
                let mut metadata = input.metadata.clone();
                metadata.insert("real_time_processor".to_string(), self.name.clone());
                metadata.insert(
                    "processing_duration_ms".to_string(),
                    duration.as_millis().to_string(),
                );
                metadata
            },
            timestamp: Utc::now(),
        };

        info!(
            "Real-time streaming processor '{}' completed processing in {:?}",
            self.name, duration
        );

        Ok(output_stream)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        Ok(*self.is_running.read().await)
    }

    async fn get_stats(&self) -> BridgeResult<StreamProcessorStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!(
            "Real-time streaming processor '{}' shutting down",
            self.name
        );

        let mut is_running = self.is_running.write().await;
        *is_running = false;

        info!(
            "Real-time streaming processor '{}' shutdown completed",
            self.name
        );
        Ok(())
    }
}

/// Streaming-aware exporter that handles real-time data
pub struct StreamingExporter {
    name: String,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ExporterStats>>,
}

impl StreamingExporter {
    pub fn new(name: String) -> Self {
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
            name,
            is_running: Arc::new(RwLock::new(true)), // Start as running
            stats: Arc::new(RwLock::new(stats)),
        }
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

    async fn stream_export(&self, batch: &ProcessedBatch) -> (bool, usize, Vec<String>) {
        // Simulate streaming export operation
        // In a real implementation, this would:
        // 1. Check if data is real-time processed
        // 2. Apply streaming-specific export logic
        // 3. Handle real-time data sinks
        // 4. Return export results

        let records_count = batch.records.len();

        // Check if this is real-time processed data
        let is_real_time = batch.metadata.get("real_time_processor").is_some();

        if is_real_time {
            info!(
                "Streaming exporter '{}' exporting real-time processed data: {} records",
                self.name, records_count
            );

            // Simulate success for real-time data
            (true, records_count, Vec::new())
        } else {
            // Simulate failure for non-real-time data
            (false, 0, vec!["Data not real-time processed".to_string()])
        }
    }
}

#[async_trait]
impl LakehouseExporter for StreamingExporter {
    async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        let start_time = Instant::now();

        // Check if exporter is running
        if !*self.is_running.read().await {
            return Err(bridge_core::error::BridgeError::export(
                "Streaming exporter is not running",
            ));
        }

        info!(
            "Streaming exporter '{}' exporting batch with {} records",
            self.name,
            batch.records.len()
        );

        // Stream export operation
        let (success, records_written, errors) = self.stream_export(&batch).await;

        let duration = start_time.elapsed();

        // Update statistics
        self.update_stats(batch.records.len(), duration, success)
            .await;

        // Convert errors to ExportError format
        let export_errors: Vec<bridge_core::types::ExportError> = errors
            .iter()
            .map(|e| bridge_core::types::ExportError {
                code: "STREAMING_EXPORT_ERROR".to_string(),
                message: e.clone(),
                details: None,
            })
            .collect();

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
            info!(
                "Streaming exporter '{}' successfully exported {} records in {:?}",
                self.name, records_written, duration
            );
        } else {
            warn!(
                "Streaming exporter '{}' failed to export some records: {} errors",
                self.name,
                errors.len()
            );
        }

        Ok(export_result)
    }

    fn name(&self) -> &str {
        &self.name
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
        info!("Streaming exporter '{}' shutting down", self.name);

        let mut is_running = self.is_running.write().await;
        *is_running = false;

        info!("Streaming exporter '{}' shutdown completed", self.name);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting streaming processor example");

    // Create pipeline configuration
    let config = PipelineConfig {
        name: "streaming-processor-pipeline".to_string(),
        max_batch_size: 1000,
        flush_interval_ms: 5000,
        buffer_size: 10000,
        enable_backpressure: true,
        backpressure_threshold: 80,
        enable_metrics: true,
        enable_health_checks: true,
        health_check_interval_ms: 30000,
    };

    // Create pipeline
    let mut pipeline = TelemetryIngestionPipeline::new(config);

    // Create and add receiver
    let mut receiver = StreamingMockReceiver::new("streaming-receiver".to_string());
    receiver.is_running = true; // Make receiver healthy
    let receiver = Arc::new(receiver);
    pipeline.add_receiver(receiver);

    // Create and add streaming exporter
    let streaming_exporter = StreamingExporter::new("streaming-exporter".to_string());
    let streaming_exporter = Arc::new(streaming_exporter);
    pipeline.add_exporter(streaming_exporter);

    // Start pipeline
    pipeline.start().await?;

    info!("Streaming processor pipeline started successfully");

    // Let it run for a few seconds
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Stop pipeline
    pipeline.stop().await?;

    info!("Streaming processor example completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_streaming_processor_creation() {
        let processor = RealTimeStreamingProcessor::new("test-processor".to_string());
        assert_eq!(processor.name(), "test-processor");
        assert_eq!(processor.version(), "1.0.0");
    }

    #[tokio::test]
    async fn test_streaming_processor_lifecycle() {
        let mut processor = RealTimeStreamingProcessor::new("test-processor".to_string());

        // Initially not running
        assert!(!processor.health_check().await.unwrap());

        // Start processor
        processor.start().await;
        assert!(processor.health_check().await.unwrap());

        // Stop processor
        processor.stop().await;
        assert!(!processor.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_streaming_exporter_creation() {
        let exporter = StreamingExporter::new("test-exporter".to_string());
        assert_eq!(exporter.name(), "test-exporter");
        assert_eq!(exporter.version(), "1.0.0");
    }

    #[tokio::test]
    async fn test_streaming_exporter_export() {
        let exporter = StreamingExporter::new("test-exporter".to_string());

        // Create a test batch with real-time processing metadata
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "test_metric".to_string(),
                description: Some("Test metric".to_string()),
                unit: Some("count".to_string()),
                metric_type: bridge_core::types::MetricType::Gauge,
                value: MetricValue::Gauge(1.0),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        let processed_record = bridge_core::types::ProcessedRecord {
            original_id: Uuid::new_v4(),
            status: bridge_core::types::ProcessingStatus::Success,
            transformed_data: Some(record.data.clone()),
            metadata: HashMap::new(),
            errors: Vec::new(),
        };

        let mut metadata = HashMap::new();
        metadata.insert(
            "real_time_processor".to_string(),
            "test-processor".to_string(),
        );

        let processed_batch = ProcessedBatch {
            original_batch_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            records: vec![processed_record],
            metadata,
            status: ProcessingStatus::Success,
            errors: Vec::new(),
        };

        // Export the batch
        let result = exporter.export(processed_batch).await.unwrap();
        assert_eq!(result.status, ExportStatus::Success);
        assert_eq!(result.records_exported, 1);
        assert_eq!(result.records_failed, 0);
    }
}
