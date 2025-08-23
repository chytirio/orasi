//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating streaming processor with bridge-core pipeline
//!
//! This example shows how to create a streaming processor that implements
//! real-time processing capabilities for telemetry data.

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
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

/// Enhanced mock receiver for streaming processor testing
#[derive(Clone)]
pub struct StreamingMockReceiver {
    name: String,
    is_running: bool,
    record_counter: Arc<RwLock<u64>>,
    data_generator: Arc<RwLock<DataGenerator>>,
}

/// Data generator for creating realistic telemetry data
pub struct DataGenerator {
    cpu_usage: f64,
    memory_usage: f64,
    error_counter: u64,
    log_levels: Vec<String>,
    error_probability: f64,
}

impl DataGenerator {
    pub fn new() -> Self {
        Self {
            cpu_usage: 50.0,
            memory_usage: 60.0,
            error_counter: 0,
            log_levels: vec!["info".to_string(), "warn".to_string(), "error".to_string()],
            error_probability: 0.1,
        }
    }

    pub fn new_with_probability(probability: f64) -> Self {
        let mut s = Self::new();
        s.error_probability = probability;
        s
    }

    pub fn generate_metrics(&mut self) -> Vec<TelemetryRecord> {
        let mut records = Vec::new();

        // Simulate CPU usage fluctuations
        self.cpu_usage += (rand::random::<f64>() - 0.5) * 20.0;
        self.cpu_usage = self.cpu_usage.clamp(10.0, 120.0);

        // Simulate memory usage
        self.memory_usage += (rand::random::<f64>() - 0.5) * 10.0;
        self.memory_usage = self.memory_usage.clamp(20.0, 95.0);

        // Generate CPU metric
        let cpu_record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "cpu_usage".to_string(),
                description: Some("CPU usage percentage".to_string()),
                unit: Some("percent".to_string()),
                metric_type: bridge_core::types::MetricType::Gauge,
                value: MetricValue::Gauge(self.cpu_usage),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };
        records.push(cpu_record);

        // Generate memory metric
        let memory_record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "memory_usage".to_string(),
                description: Some("Memory usage percentage".to_string()),
                unit: Some("percent".to_string()),
                metric_type: bridge_core::types::MetricType::Gauge,
                value: MetricValue::Gauge(self.memory_usage),
                labels: HashMap::new(),
                timestamp: Utc::now(),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };
        records.push(memory_record);

        // Generate log record (occasionally with errors)
        if rand::random::<f64>() < self.error_probability {
            self.error_counter += 1;
            let log_record = TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Log,
                data: TelemetryData::Log(bridge_core::types::LogData {
                    timestamp: Utc::now(),
                    level: bridge_core::types::LogLevel::Error,
                    message: format!("Database connection failed (error #{})", self.error_counter),
                    attributes: HashMap::new(),
                    body: Some(format!(
                        "Database connection failed (error #{})",
                        self.error_counter
                    )),
                    severity_number: Some(17), // ERROR level
                    severity_text: Some("ERROR".to_string()),
                }),
                attributes: HashMap::new(),
                tags: HashMap::new(),
                resource: None,
                service: None,
            };
            records.push(log_record);
        } else {
            // Generate info log
            let log_record = TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Log,
                data: TelemetryData::Log(bridge_core::types::LogData {
                    timestamp: Utc::now(),
                    level: bridge_core::types::LogLevel::Info,
                    message: "Application running normally".to_string(),
                    attributes: HashMap::new(),
                    body: Some("Application running normally".to_string()),
                    severity_number: Some(9), // INFO level
                    severity_text: Some("INFO".to_string()),
                }),
                attributes: HashMap::new(),
                tags: HashMap::new(),
                resource: None,
                service: None,
            };
            records.push(log_record);
        }

        records
    }
}

impl StreamingMockReceiver {
    pub fn new(name: String, error_probability: f64) -> Self {
        Self {
            name,
            is_running: false,
            record_counter: Arc::new(RwLock::new(0)),
            data_generator: Arc::new(RwLock::new(DataGenerator::new_with_probability(
                error_probability,
            ))),
        }
    }
}

#[async_trait]
impl TelemetryReceiver for StreamingMockReceiver {
    async fn receive(&self) -> BridgeResult<bridge_core::types::TelemetryBatch> {
        // Generate realistic telemetry data
        let mut generator = self.data_generator.write().await;
        let records = generator.generate_metrics();
        {
            let mut counter = self.record_counter.write().await;
            *counter += records.len() as u64;
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
        let counter = self.record_counter.read().await;
        Ok(bridge_core::traits::ReceiverStats {
            total_records: *counter,
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

/// Enhanced streaming-aware exporter that handles real-time data
pub struct StreamingExporter {
    name: String,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ExporterStats>>,
    http_sink: Option<Arc<tokio::sync::RwLock<HttpSink>>>,
}

/// Simple HTTP sink for testing
pub struct HttpSink {
    endpoint_url: String,
    is_initialized: bool,
    is_running: bool,
}

impl HttpSink {
    pub fn new(endpoint_url: String) -> Self {
        Self {
            endpoint_url,
            is_initialized: false,
            is_running: false,
        }
    }

    pub async fn init(&mut self) -> BridgeResult<()> {
        info!("Initializing HTTP sink: {}", self.endpoint_url);
        self.is_initialized = true;
        Ok(())
    }

    pub async fn start(&mut self) -> BridgeResult<()> {
        if !self.is_initialized {
            return Err(bridge_core::error::BridgeError::internal(
                "HTTP sink not initialized".to_string(),
            ));
        }
        info!("Starting HTTP sink: {}", self.endpoint_url);
        self.is_running = true;
        Ok(())
    }

    pub async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping HTTP sink: {}", self.endpoint_url);
        self.is_running = false;
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub async fn send(&self, _data: &str) -> BridgeResult<()> {
        if !self.is_running {
            return Err(bridge_core::error::BridgeError::internal(
                "HTTP sink is not running".to_string(),
            ));
        }
        // Simulate HTTP request
        info!("HTTP sink '{}' sending data (simulated)", self.endpoint_url);
        Ok(())
    }
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
            http_sink: None,
        }
    }

    pub async fn add_http_sink(&mut self, endpoint_url: String) -> BridgeResult<()> {
        let mut http_sink = HttpSink::new(endpoint_url);
        http_sink.init().await?;
        http_sink.start().await?;

        self.http_sink = Some(Arc::new(tokio::sync::RwLock::new(http_sink)));
        info!("HTTP sink added and started for exporter '{}'", self.name);
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

    async fn stream_export(&self, batch: &ProcessedBatch) -> (bool, usize, Vec<String>) {
        let records_count = batch.records.len();
        let mut errors = Vec::new();

        // Check if this is real-time processed data
        let is_real_time = batch.metadata.get("real_time_processor").is_some();

        if is_real_time {
            info!(
                "Streaming exporter '{}' exporting real-time processed data: {} records",
                self.name, records_count
            );

            // Try to send to HTTP sink if available
            if let Some(http_sink) = &self.http_sink {
                let sink = http_sink.read().await;
                if let Err(e) = sink.send("simulated_data").await {
                    errors.push(format!("HTTP sink error: {}", e));
                }
            }

            // Simulate success for real-time data
            (errors.is_empty(), records_count, errors)
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

        // Stop HTTP sink if available
        if let Some(http_sink) = &self.http_sink {
            let mut sink = http_sink.write().await;
            if let Err(e) = sink.stop().await {
                warn!("Failed to stop HTTP sink: {}", e);
            }
        }

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

    info!("Starting enhanced streaming processor example");

    // Load configuration from file (TOML)
    #[derive(Debug, Deserialize)]
    struct ExampleConfig {
        run_seconds: Option<u64>,
        http_sink_endpoint: Option<String>,
        receiver_healthy: Option<bool>,
        error_probability: Option<f64>,
    }

    let cfg_path = std::env::var("ORASI_STREAMING_EXAMPLE_CONFIG")
        .unwrap_or_else(|_| "config/streaming_example.toml".to_string());
    let example_cfg: ExampleConfig = match fs::read_to_string(&cfg_path) {
        Ok(contents) => match toml::from_str(&contents) {
            Ok(cfg) => cfg,
            Err(err) => {
                warn!(
                    "Failed to parse {} as TOML ({}). Falling back to defaults.",
                    cfg_path, err
                );
                ExampleConfig {
                    run_seconds: None,
                    http_sink_endpoint: None,
                    receiver_healthy: None,
                    error_probability: None,
                }
            }
        },
        Err(_) => {
            warn!("Config file {} not found. Using defaults.", cfg_path);
            ExampleConfig {
                run_seconds: None,
                http_sink_endpoint: None,
                receiver_healthy: None,
                error_probability: None,
            }
        }
    };

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
    let mut receiver = StreamingMockReceiver::new(
        "streaming-receiver".to_string(),
        example_cfg.error_probability.unwrap_or(0.1),
    );
    receiver.is_running = example_cfg.receiver_healthy.unwrap_or(true); // Make receiver healthy
    let receiver = Arc::new(receiver);
    pipeline.add_receiver(receiver);

    // Create and add streaming exporter with HTTP sink
    let mut streaming_exporter = StreamingExporter::new("streaming-exporter".to_string());

    // Add and initialize HTTP sink
    streaming_exporter
        .add_http_sink(
            example_cfg
                .http_sink_endpoint
                .unwrap_or_else(|| "http://localhost:8080/api/telemetry".to_string()),
        )
        .await?;

    let streaming_exporter = Arc::new(streaming_exporter);
    pipeline.add_exporter(streaming_exporter);

    // Start pipeline
    pipeline.start().await?;

    info!("Enhanced streaming processor pipeline started successfully");

    // Let it run for a configured number of seconds to generate data
    let run_secs = example_cfg.run_seconds.unwrap_or(5);
    tokio::time::sleep(Duration::from_secs(run_secs)).await;

    // Stop pipeline
    pipeline.stop().await?;

    info!("Enhanced streaming processor example completed successfully");
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

    #[tokio::test]
    async fn test_data_generator() {
        let mut generator = DataGenerator::new();
        let records = generator.generate_metrics();

        assert!(!records.is_empty());
        assert!(records.len() >= 2); // Should have at least CPU and memory metrics
    }

    #[tokio::test]
    async fn test_http_sink_lifecycle() {
        let mut http_sink = HttpSink::new("http://localhost:8080/test".to_string());

        // Initially not initialized
        assert!(!http_sink.is_initialized);

        // Initialize
        http_sink.init().await.unwrap();
        assert!(http_sink.is_initialized);

        // Start
        http_sink.start().await.unwrap();
        assert!(http_sink.is_running());

        // Stop
        http_sink.stop().await.unwrap();
        assert!(!http_sink.is_running());
    }
}
