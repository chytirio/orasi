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
        
        tracing::info!("Starting streaming processor with config: {:?}", self.config);
        
        // Initialize sources
        self.initialize_sources().await?;
        
        // Initialize processors
        self.initialize_processors().await?;
        
        // Initialize sinks
        self.initialize_sinks().await?;
        
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
        // For now, we'll create a simple mock source since the config types don't match
        // In a real implementation, you would convert the config to the appropriate source types
        tracing::info!("Initializing sources (placeholder implementation)");
        Ok(())
    }

    /// Initialize processors based on configuration
    async fn initialize_processors(&mut self) -> bridge_core::BridgeResult<()> {
        // For now, we'll create a simple mock processor since the config types don't match
        // In a real implementation, you would convert the config to the appropriate processor types
        tracing::info!("Initializing processors (placeholder implementation)");
        Ok(())
    }

    /// Initialize sinks based on configuration
    async fn initialize_sinks(&mut self) -> bridge_core::BridgeResult<()> {
        // For now, we'll create a simple mock sink since the config types don't match
        // In a real implementation, you would convert the config to the appropriate sink types
        tracing::info!("Initializing sinks (placeholder implementation)");
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

    async fn get_stats(&self) -> bridge_core::BridgeResult<bridge_core::traits::StreamProcessorStats> {
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
