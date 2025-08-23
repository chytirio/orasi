//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup@gmail.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming processor for the bridge
//!
//! This crate provides a streaming data processor that can:
//! - Ingest data from various sources (HTTP, Kafka, files)
//! - Process data through configurable pipelines
//! - Output data to various sinks (HTTP, Kafka, files)
//! - Support filtering, transformation, and aggregation

pub mod aggregations;
pub mod arrow_utils;
pub mod config;
pub mod metrics;
pub mod processors;
pub mod sinks;
pub mod sources;
pub mod state;
pub mod transformations;
pub mod windows;

// Re-export main types
pub use config::StreamingProcessorConfig;
pub use processors::{ProcessorFactory, ProcessorPipeline};
pub use sinks::{SinkFactory, SinkManager};
pub use sources::{SourceFactory, SourceManager};

// Re-export bridge-core traits for convenience
pub use bridge_core::traits::{StreamProcessor, StreamSink, StreamSource};

/// Main streaming processor implementation
pub struct StreamingProcessor {
    config: StreamingProcessorConfig,
    source_manager: SourceManager,
    processor_pipeline: ProcessorPipeline,
    sink_manager: SinkManager,
    is_running: std::sync::Arc<tokio::sync::RwLock<bool>>,
    stats: std::sync::Arc<tokio::sync::RwLock<bridge_core::traits::StreamProcessorStats>>,
}

impl StreamingProcessor {
    /// Create a new streaming processor
    pub async fn new(config: StreamingProcessorConfig) -> bridge_core::BridgeResult<Self> {
        let source_manager = SourceManager::new();
        let processor_pipeline = ProcessorPipeline::new();
        let sink_manager = SinkManager::new();

        let stats = bridge_core::traits::StreamProcessorStats {
            total_records: 0,
            records_per_minute: 0,
            avg_processing_time_ms: 0.0,
            error_count: 0,
            last_process_time: None,
        };

        Ok(Self {
            config,
            source_manager,
            processor_pipeline,
            sink_manager,
            is_running: std::sync::Arc::new(tokio::sync::RwLock::new(false)),
            stats: std::sync::Arc::new(tokio::sync::RwLock::new(stats)),
        })
    }

    /// Start the streaming processor
    pub async fn start(&mut self) -> bridge_core::BridgeResult<()> {
        {
            let mut is_running = self.is_running.write().await;
            *is_running = true;
        }

        tracing::info!(
            "Starting streaming processor with config: {:?}",
            self.config
        );

        // Initialize sources
        self.initialize_sources().await?;

        // Initialize processors
        self.initialize_processors().await?;

        // Initialize sinks
        self.initialize_sinks().await?;

        // Start all components
        self.source_manager.start_all().await?;
        // Note: ProcessorPipeline doesn't need to be started - it processes on demand
        self.sink_manager.start_all().await?;

        tracing::info!("Streaming processor started successfully");
        Ok(())
    }

    /// Stop the streaming processor
    pub async fn stop(&mut self) -> bridge_core::BridgeResult<()> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;

        tracing::info!("Stopping streaming processor");

        // Stop sources
        self.source_manager.stop_all().await?;

        // Stop processors
        self.processor_pipeline.shutdown().await?;

        // Stop sinks
        self.sink_manager.stop_all().await?;

        tracing::info!("Streaming processor stopped successfully");
        Ok(())
    }

    /// Initialize sources based on configuration
    async fn initialize_sources(&mut self) -> bridge_core::BridgeResult<()> {
        tracing::info!("Initializing sources");

        // Create sources based on configuration
        for (source_name, source_config) in &self.config.sources {
            match &source_config.source_type {
                config::SourceType::Kafka => {
                    let endpoint = source_config
                        .config
                        .get("endpoint")
                        .and_then(|v| v.as_str())
                        .unwrap_or("localhost:9092");
                    let topic = source_config
                        .config
                        .get("topic")
                        .and_then(|v| v.as_str())
                        .unwrap_or("default-topic");

                    let kafka_config = sources::kafka_source::KafkaSourceConfig::new(
                        vec![endpoint.to_string()],
                        topic.to_string(),
                        "default-group".to_string(),
                    );
                    let source = sources::kafka_source::KafkaSource::new(&kafka_config).await?;
                    self.source_manager
                        .add_source(source_name.clone(), Box::new(source));
                }
                config::SourceType::Http => {
                    let endpoint = source_config
                        .config
                        .get("endpoint")
                        .and_then(|v| v.as_str())
                        .unwrap_or("http://localhost:8080");

                    let http_config =
                        sources::http_source::HttpSourceConfig::new(endpoint.to_string());
                    let source = sources::http_source::HttpSource::new(&http_config).await?;
                    self.source_manager
                        .add_source(source_name.clone(), Box::new(source));
                }
                config::SourceType::File => {
                    let file_path = source_config
                        .config
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("/tmp/data.json");
                    let format_str = source_config
                        .config
                        .get("format")
                        .and_then(|v| v.as_str())
                        .unwrap_or("json");

                    let file_format = match format_str {
                        "json" => sources::file_source::FileFormat::Json,
                        "csv" => sources::file_source::FileFormat::Csv,
                        "parquet" => sources::file_source::FileFormat::Parquet,
                        "avro" => sources::file_source::FileFormat::Avro,
                        "arrow" => sources::file_source::FileFormat::Arrow,
                        _ => sources::file_source::FileFormat::Json,
                    };

                    let file_config = sources::file_source::FileSourceConfig::new(
                        file_path.to_string(),
                        file_format,
                    );
                    let source = sources::file_source::FileSource::new(&file_config).await?;
                    self.source_manager
                        .add_source(source_name.clone(), Box::new(source));
                }
                config::SourceType::WebSocket => {
                    tracing::warn!("WebSocket sources not yet implemented");
                }
                config::SourceType::Custom(custom_type) => {
                    tracing::warn!("Custom source type '{}' not yet implemented", custom_type);
                }
            }
        }

        tracing::info!("Sources initialized successfully");
        Ok(())
    }

    /// Initialize processors based on configuration
    async fn initialize_processors(&mut self) -> bridge_core::BridgeResult<()> {
        tracing::info!("Initializing processors");

        // Create processors based on configuration
        for processor_config in &self.config.processors {
            match &processor_config.processor_type {
                config::ProcessorType::Filter => {
                    let conditions = processor_config
                        .config
                        .get("conditions")
                        .and_then(|v| v.as_object())
                        .map(|obj| {
                            obj.iter()
                                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                                .collect::<std::collections::HashMap<String, String>>()
                        })
                        .unwrap_or_default();

                    // Create filter rules from conditions
                    let filter_rules = conditions
                        .iter()
                        .map(|(field, value)| processors::filter_processor::FilterRule {
                            name: field.clone(),
                            field: field.clone(),
                            operator: processors::filter_processor::FilterOperator::Equals,
                            value: value.clone(),
                            enabled: true,
                        })
                        .collect();

                    let filter_config = processors::filter_processor::FilterProcessorConfig::new(
                        filter_rules,
                        processors::filter_processor::FilterMode::Include,
                    );
                    let filter_processor =
                        processors::filter_processor::FilterProcessor::new(&filter_config).await?;
                    self.processor_pipeline
                        .add_processor(Box::new(filter_processor));
                }
                config::ProcessorType::Transform => {
                    let transformations = processor_config
                        .config
                        .get("transformations")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default();

                    // Create transform rules from transformations
                    let transform_rules = transformations
                        .iter()
                        .map(|transform| processors::transform_processor::TransformRule {
                            name: transform.clone(),
                            rule_type: processors::transform_processor::TransformRuleType::Copy,
                            source_field: transform.clone(),
                            target_field: transform.clone(),
                            transform_value: None,
                            enabled: true,
                        })
                        .collect();

                    let transform_config =
                        processors::transform_processor::TransformProcessorConfig::new(
                            transform_rules,
                        );
                    let transform_processor =
                        processors::transform_processor::TransformProcessor::new(&transform_config)
                            .await?;
                    self.processor_pipeline
                        .add_processor(Box::new(transform_processor));
                }
                config::ProcessorType::Aggregate => {
                    let aggregations = processor_config
                        .config
                        .get("aggregations")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default();

                    // Create aggregation rules from aggregations
                    let aggregation_rules = aggregations.iter().map(|aggregation| {
                        processors::aggregate_processor::AggregationRule {
                            name: aggregation.clone(),
                            group_by_fields: vec![aggregation.clone()],
                            aggregation_functions: vec![
                                processors::aggregate_processor::AggregationFunction {
                                    name: "count".to_string(),
                                    source_field: aggregation.clone(),
                                    target_field: format!("{}_count", aggregation),
                                    function_type: processors::aggregate_processor::AggregationFunctionType::Count,
                                }
                            ],
                            enabled: true,
                        }
                    }).collect();

                    let aggregate_config =
                        processors::aggregate_processor::AggregateProcessorConfig::new(
                            aggregation_rules,
                            60000,
                        ); // 1 minute window
                    let aggregate_processor =
                        processors::aggregate_processor::AggregateProcessor::new(&aggregate_config)
                            .await?;
                    self.processor_pipeline
                        .add_processor(Box::new(aggregate_processor));
                }
                config::ProcessorType::Window => {
                    tracing::warn!("Window processor not yet implemented");
                }
                config::ProcessorType::Custom(custom_type) => {
                    tracing::warn!(
                        "Custom processor type '{}' not yet implemented",
                        custom_type
                    );
                }
            }
        }

        tracing::info!("Processors initialized successfully");
        Ok(())
    }

    /// Initialize sinks based on configuration
    async fn initialize_sinks(&mut self) -> bridge_core::BridgeResult<()> {
        tracing::info!("Initializing sinks");

        // Create sinks based on configuration
        for (sink_name, sink_config) in &self.config.sinks {
            match &sink_config.sink_type {
                config::SinkType::Kafka => {
                    let endpoint = sink_config
                        .config
                        .get("endpoint")
                        .and_then(|v| v.as_str())
                        .unwrap_or("localhost:9092");
                    let topic = sink_config
                        .config
                        .get("topic")
                        .and_then(|v| v.as_str())
                        .unwrap_or("default-topic");

                    let kafka_config = sinks::kafka_sink::KafkaSinkConfig::new(
                        vec![endpoint.to_string()],
                        topic.to_string(),
                    );
                    let kafka_sink = sinks::kafka_sink::KafkaSink::new(&kafka_config).await?;
                    self.sink_manager
                        .add_sink(sink_name.clone(), Box::new(kafka_sink));
                }
                config::SinkType::Http => {
                    let endpoint = sink_config
                        .config
                        .get("endpoint")
                        .and_then(|v| v.as_str())
                        .unwrap_or("http://localhost:8080");

                    let http_config = sinks::http_sink::HttpSinkConfig::new(endpoint.to_string());
                    let http_sink = sinks::http_sink::HttpSink::new(&http_config).await?;
                    self.sink_manager
                        .add_sink(sink_name.clone(), Box::new(http_sink));
                }
                config::SinkType::File => {
                    let file_path = sink_config
                        .config
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("/tmp/output.json");
                    let format_str = sink_config
                        .config
                        .get("format")
                        .and_then(|v| v.as_str())
                        .unwrap_or("json");

                    let file_format = match format_str {
                        "json" => sinks::file_sink::FileFormat::Json,
                        "csv" => sinks::file_sink::FileFormat::Csv,
                        "parquet" => sinks::file_sink::FileFormat::Parquet,
                        "avro" => sinks::file_sink::FileFormat::Avro,
                        "arrow" => sinks::file_sink::FileFormat::Arrow,
                        _ => sinks::file_sink::FileFormat::Json,
                    };

                    let file_config =
                        sinks::file_sink::FileSinkConfig::new(file_path.to_string(), file_format);
                    let file_sink = sinks::file_sink::FileSink::new(&file_config).await?;
                    self.sink_manager
                        .add_sink(sink_name.clone(), Box::new(file_sink));
                }
                config::SinkType::Database => {
                    tracing::warn!("Database sinks not yet implemented");
                }
                config::SinkType::Custom(custom_type) => {
                    tracing::warn!("Custom sink type '{}' not yet implemented", custom_type);
                }
            }
        }

        tracing::info!("Sinks initialized successfully");
        Ok(())
    }

    /// Process data through the pipeline
    pub async fn process_data(
        &mut self,
        data: bridge_core::TelemetryBatch,
    ) -> bridge_core::BridgeResult<()> {
        let start_time = std::time::Instant::now();

        // Convert TelemetryBatch to DataStream for processing
        let data_stream = bridge_core::traits::DataStream {
            stream_id: format!("batch_{}", data.id),
            data: serde_json::to_vec(&data).map_err(|e| {
                bridge_core::BridgeError::internal(format!("Failed to serialize batch: {}", e))
            })?,
            metadata: data.metadata.clone(),
            timestamp: chrono::Utc::now(),
        };

        // Process through pipeline
        let processed_stream = self.processor_pipeline.process_stream(data_stream).await?;

        // Convert back to TelemetryBatch for sinks
        let processed_data: bridge_core::TelemetryBatch =
            serde_json::from_slice(&processed_stream.data).map_err(|e| {
                bridge_core::BridgeError::internal(format!(
                    "Failed to deserialize processed data: {}",
                    e
                ))
            })?;

        // Send to sinks
        self.sink_manager
            .send_to_all(processed_data.clone())
            .await?;

        let duration = start_time.elapsed();

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_records += processed_data.size as u64;
            stats.last_process_time = Some(chrono::Utc::now());

            let duration_ms = duration.as_millis() as f64;
            if stats.total_records > processed_data.size as u64 {
                stats.avg_processing_time_ms = (stats.avg_processing_time_ms
                    * (stats.total_records - processed_data.size as u64) as f64
                    + duration_ms)
                    / stats.total_records as f64;
            } else {
                stats.avg_processing_time_ms = duration_ms;
            }
        }

        Ok(())
    }

    /// Get processor statistics
    pub async fn get_stats(&self) -> bridge_core::traits::StreamProcessorStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get source manager
    pub fn source_manager(&self) -> &SourceManager {
        &self.source_manager
    }

    /// Get processor pipeline
    pub fn processor_pipeline(&self) -> &ProcessorPipeline {
        &self.processor_pipeline
    }

    /// Get sink manager
    pub fn sink_manager(&self) -> &SinkManager {
        &self.sink_manager
    }

    /// Check if processor is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
}

/// Simple streaming processor implementation (legacy)
pub struct SimpleStreamingProcessor {
    name: String,
    is_running: std::sync::Arc<tokio::sync::RwLock<bool>>,
    stats: std::sync::Arc<tokio::sync::RwLock<bridge_core::traits::StreamProcessorStats>>,
}

impl SimpleStreamingProcessor {
    /// Create a new simple streaming processor
    pub fn new(name: String) -> Self {
        let stats = bridge_core::traits::StreamProcessorStats {
            total_records: 0,
            records_per_minute: 0,
            avg_processing_time_ms: 0.0,
            error_count: 0,
            last_process_time: None,
        };

        Self {
            name,
            is_running: std::sync::Arc::new(tokio::sync::RwLock::new(false)),
            stats: std::sync::Arc::new(tokio::sync::RwLock::new(stats)),
        }
    }

    /// Start the processor
    pub async fn start(&mut self) {
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        tracing::info!("Simple streaming processor '{}' started", self.name);
    }

    /// Stop the processor
    pub async fn stop(&mut self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        tracing::info!("Simple streaming processor '{}' stopped", self.name);
    }
}

#[async_trait::async_trait]
impl bridge_core::traits::StreamProcessor for SimpleStreamingProcessor {
    async fn process_stream(
        &self,
        input: bridge_core::traits::DataStream,
    ) -> bridge_core::BridgeResult<bridge_core::traits::DataStream> {
        let start_time = std::time::Instant::now();

        // Check if processor is running
        if !*self.is_running.read().await {
            return Err(bridge_core::error::BridgeError::internal(
                "Simple streaming processor is not running",
            ));
        }

        tracing::info!(
            "Simple streaming processor '{}' processing stream: {}",
            self.name,
            input.stream_id
        );

        // Simple processing: add metadata to indicate processing
        let mut metadata = input.metadata.clone();
        metadata.insert("processed_by".to_string(), self.name.clone());
        metadata.insert(
            "processing_timestamp".to_string(),
            chrono::Utc::now().to_rfc3339(),
        );

        let duration = start_time.elapsed();

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_records += input.data.len() as u64;
            stats.last_process_time = Some(chrono::Utc::now());

            let duration_ms = duration.as_millis() as f64;
            if stats.total_records > input.data.len() as u64 {
                stats.avg_processing_time_ms = (stats.avg_processing_time_ms
                    * (stats.total_records - input.data.len() as u64) as f64
                    + duration_ms)
                    / stats.total_records as f64;
            } else {
                stats.avg_processing_time_ms = duration_ms;
            }
        }

        // Create processed output stream
        let output_stream = bridge_core::traits::DataStream {
            stream_id: format!("processed_{}", input.stream_id),
            data: input.data, // In real implementation, this would be transformed data
            metadata,
            timestamp: chrono::Utc::now(),
        };

        tracing::info!(
            "Simple streaming processor '{}' completed processing in {:?}",
            self.name,
            duration
        );

        Ok(output_stream)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn health_check(&self) -> bridge_core::BridgeResult<bool> {
        Ok(*self.is_running.read().await)
    }

    async fn get_stats(
        &self,
    ) -> bridge_core::BridgeResult<bridge_core::traits::StreamProcessorStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> bridge_core::BridgeResult<()> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        tracing::info!("Simple streaming processor '{}' shutdown", self.name);
        Ok(())
    }
}

/// Streaming processor version
pub const STREAMING_PROCESSOR_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default streaming endpoint
pub const DEFAULT_STREAMING_ENDPOINT: &str = "0.0.0.0:8080";

/// Default Kafka endpoint
pub const DEFAULT_KAFKA_ENDPOINT: &str = "localhost:9092";

/// Default batch size
pub const DEFAULT_BATCH_SIZE: usize = 1000;

/// Default window size in milliseconds
pub const DEFAULT_WINDOW_SIZE_MS: u64 = 60000;

/// Default flush interval in milliseconds
pub const DEFAULT_FLUSH_INTERVAL_MS: u64 = 5000;

/// Default buffer size
pub const DEFAULT_BUFFER_SIZE: usize = 10000;

/// Initialize streaming processor with default configuration
pub async fn init_streaming_processor() -> bridge_core::BridgeResult<StreamingProcessor> {
    let config = StreamingProcessorConfig::default();
    StreamingProcessor::new(config).await
}

/// Initialize streaming processor with custom configuration
pub async fn init_streaming_processor_with_config(
    config: StreamingProcessorConfig,
) -> bridge_core::BridgeResult<StreamingProcessor> {
    StreamingProcessor::new(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_streaming_processor_initialization() {
        let result = init_streaming_processor().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_streaming_processor_shutdown() {
        let processor = init_streaming_processor().await.unwrap();
        let result = processor.processor_pipeline.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_simple_streaming_processor_creation() {
        let processor = SimpleStreamingProcessor::new("test-processor".to_string());
        assert_eq!(processor.name(), "test-processor");
        assert_eq!(processor.version(), "1.0.0");
    }

    #[tokio::test]
    async fn test_simple_streaming_processor_lifecycle() {
        let mut processor = SimpleStreamingProcessor::new("test-processor".to_string());

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
    async fn test_simple_streaming_processor_processing() {
        let mut processor = SimpleStreamingProcessor::new("test-processor".to_string());
        processor.start().await;

        let input_stream = bridge_core::traits::DataStream {
            stream_id: "test-stream".to_string(),
            data: vec![1, 2, 3, 4, 5],
            metadata: std::collections::HashMap::new(),
            timestamp: chrono::Utc::now(),
        };

        let result = processor.process_stream(input_stream).await;
        assert!(result.is_ok());

        let output_stream = result.unwrap();
        assert_eq!(output_stream.stream_id, "processed_test-stream");
        assert!(output_stream.metadata.contains_key("processed_by"));
        assert!(output_stream.metadata.contains_key("processing_timestamp"));
    }
}
