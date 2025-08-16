//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming data processors for the bridge
//!
//! This module provides processor implementations for streaming data
//! including filtering, transformation, and aggregation.

pub mod aggregate_processor;
pub mod filter_processor;
pub mod stream_processor;
pub mod transform_processor;

// Re-export processor implementations
pub use aggregate_processor::AggregateProcessor;
pub use filter_processor::FilterProcessor;
pub use stream_processor::StreamProcessor;
pub use transform_processor::TransformProcessor;

use async_trait::async_trait;
use bridge_core::{
    traits::{DataStream, StreamProcessor as BridgeStreamProcessor, StreamProcessorStats},
    BridgeResult,
};
use chrono::Utc;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// Processor configuration trait
#[async_trait]
pub trait ProcessorConfig: Send + Sync {
    /// Get processor name
    fn name(&self) -> &str;

    /// Get processor version
    fn version(&self) -> &str;

    /// Validate configuration
    async fn validate(&self) -> BridgeResult<()>;

    /// Get configuration as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Processor factory for creating processors
pub struct ProcessorFactory;

impl ProcessorFactory {
    /// Create a processor based on configuration
    pub async fn create_processor(
        config: &dyn ProcessorConfig,
    ) -> BridgeResult<Box<dyn crate::StreamProcessor>> {
        match config.name() {
            "stream" => {
                let processor = StreamProcessor::new(config).await?;
                Ok(Box::new(processor))
            }
            "filter" => {
                let processor = FilterProcessor::new(config).await?;
                Ok(Box::new(processor))
            }
            "transform" => {
                let processor = TransformProcessor::new(config).await?;
                Ok(Box::new(processor))
            }
            "aggregate" => {
                let processor = AggregateProcessor::new(config).await?;
                Ok(Box::new(processor))
            }
            _ => Err(bridge_core::BridgeError::configuration(format!(
                "Unsupported processor: {}",
                config.name()
            ))),
        }
    }
}

/// Processor pipeline for chaining multiple processors
pub struct ProcessorPipeline {
    processors: Vec<Box<dyn BridgeStreamProcessor>>,
    stats: Arc<RwLock<StreamProcessorStats>>,
}

impl ProcessorPipeline {
    /// Create new processor pipeline
    pub fn new() -> Self {
        let stats = StreamProcessorStats {
            total_records: 0,
            records_per_minute: 0,
            avg_processing_time_ms: 0.0,
            error_count: 0,
            last_process_time: None,
        };

        Self {
            processors: Vec::new(),
            stats: Arc::new(RwLock::new(stats)),
        }
    }

    /// Add processor to pipeline
    pub fn add_processor(&mut self, processor: Box<dyn BridgeStreamProcessor>) {
        info!("Adding processor to pipeline: {}", processor.name());
        self.processors.push(processor);
    }

    /// Remove processor from pipeline
    pub fn remove_processor(&mut self, index: usize) -> Option<Box<dyn BridgeStreamProcessor>> {
        if index < self.processors.len() {
            let processor = self.processors.remove(index);
            info!("Removed processor from pipeline: {}", processor.name());
            Some(processor)
        } else {
            None
        }
    }

    /// Get processor by index
    pub fn get_processor(&self, index: usize) -> Option<&dyn BridgeStreamProcessor> {
        self.processors.get(index).map(|p| p.as_ref())
    }

    /// Get number of processors
    pub fn len(&self) -> usize {
        self.processors.len()
    }

    /// Check if pipeline is empty
    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }

    /// Process data stream through the pipeline
    pub async fn process_stream(&self, input: DataStream) -> BridgeResult<DataStream> {
        let start_time = std::time::Instant::now();
        let mut current_stream = input;

        for (i, processor) in self.processors.iter().enumerate() {
            info!(
                "Processing stream through processor {}: {}",
                i,
                processor.name()
            );

            match processor.process_stream(current_stream).await {
                Ok(processed_stream) => {
                    current_stream = processed_stream;
                    info!(
                        "Stream processed through processor {}: {}",
                        i,
                        processor.name()
                    );
                }
                Err(e) => {
                    error!("Failed to process stream through processor {}: {}", i, e);
                    return Err(e);
                }
            }
        }

        let processing_time = start_time.elapsed().as_millis() as f64;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_records += 1; // Simplified - in practice would count actual records
            stats.avg_processing_time_ms = (stats.avg_processing_time_ms + processing_time) / 2.0;
            stats.last_process_time = Some(Utc::now());
        }

        info!("Stream processed through pipeline in {}ms", processing_time);
        Ok(current_stream)
    }

    /// Get pipeline statistics
    pub async fn get_stats(&self) -> BridgeResult<StreamProcessorStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    /// Health check for all processors
    pub async fn health_check(&self) -> BridgeResult<bool> {
        for (i, processor) in self.processors.iter().enumerate() {
            match processor.health_check().await {
                Ok(healthy) => {
                    if !healthy {
                        error!("Processor {} ({}) is unhealthy", i, processor.name());
                        return Ok(false);
                    }
                }
                Err(e) => {
                    error!("Failed to check health of processor {}: {}", i, e);
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// Shutdown all processors
    pub async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down processor pipeline");

        for (i, processor) in self.processors.iter().enumerate() {
            info!("Shutting down processor {}: {}", i, processor.name());
            if let Err(e) = processor.shutdown().await {
                error!("Failed to shutdown processor {}: {}", i, e);
                return Err(e);
            }
        }

        info!("Processor pipeline shutdown complete");
        Ok(())
    }
}
