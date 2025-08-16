//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry processors for OpenTelemetry data ingestion
//!
//! This module provides processor implementations for processing and transforming
//! telemetry data during ingestion.

pub mod batch_processor;
pub mod enrichment_processor;
pub mod filter_processor;
pub mod transform_processor;

// Re-export processor implementations
pub use batch_processor::BatchProcessor;
pub use enrichment_processor::{EnrichmentProcessor, EnrichmentProcessorConfig};
pub use filter_processor::FilterProcessor;
pub use transform_processor::TransformProcessor;

use async_trait::async_trait;
use bridge_core::{
    traits::ProcessorStats,
    types::{ProcessedRecord, TelemetryData, TelemetryRecord, TelemetryType},
    BridgeResult, ProcessedBatch, TelemetryBatch, TelemetryProcessor,
};
use chrono::Utc;
use std::any::Any;
use std::collections::HashMap;
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

/// Base processor implementation
pub struct BaseProcessor {
    name: String,
    version: String,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ProcessorStats>>,
}

impl BaseProcessor {
    /// Create new base processor
    pub fn new(name: String, version: String) -> Self {
        let stats = ProcessorStats {
            total_batches: 0,
            total_records: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            avg_processing_time_ms: 0.0,
            error_count: 0,
            last_process_time: None,
        };

        Self {
            name,
            version,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
        }
    }

    /// Update processor statistics
    pub async fn update_stats(&self, batches: usize, records: usize, processing_time_ms: f64) {
        let mut stats = self.stats.write().await;
        stats.total_batches += batches as u64;
        stats.total_records += records as u64;
        stats.last_process_time = Some(Utc::now());

        // Update average processing time
        if stats.total_batches > 0 {
            let total_time = stats.avg_processing_time_ms * (stats.total_batches - 1) as f64
                + processing_time_ms;
            stats.avg_processing_time_ms = total_time / stats.total_batches as f64;
        }
    }

    /// Increment error count
    pub async fn increment_error_count(&self) {
        let mut stats = self.stats.write().await;
        stats.error_count += 1;
    }

    /// Get processor name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get processor version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get processor statistics
    pub async fn get_stats(&self) -> BridgeResult<ProcessorStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}

/// Processor factory for creating processors
pub struct ProcessorFactory;

impl ProcessorFactory {
    /// Create a processor based on configuration
    pub async fn create_processor(
        config: &dyn ProcessorConfig,
    ) -> BridgeResult<Box<dyn TelemetryProcessor>> {
        match config.name() {
            "batch" => {
                let processor = BatchProcessor::new(config).await?;
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
            "enrichment" => {
                let processor = EnrichmentProcessor::new(config).await?;
                Ok(Box::new(processor) as Box<dyn TelemetryProcessor>)
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
    processors: Vec<Box<dyn TelemetryProcessor>>,
    is_running: Arc<RwLock<bool>>,
}

impl ProcessorPipeline {
    /// Create new processor pipeline
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Add processor to pipeline
    pub fn add_processor(&mut self, processor: Box<dyn TelemetryProcessor>) {
        self.processors.push(processor);
    }

    /// Process batch through pipeline
    pub async fn process(&self, batch: TelemetryBatch) -> BridgeResult<ProcessedBatch> {
        let mut current_batch = batch;

        for processor in &self.processors {
            let start_time = std::time::Instant::now();

            match processor.process(current_batch.clone()).await {
                Ok(processed_batch) => {
                    // Handle processed batch correctly
                    // Convert ProcessedBatch back to TelemetryBatch for next processor
                    current_batch = TelemetryBatch {
                        id: processed_batch.original_batch_id,
                        timestamp: processed_batch.timestamp,
                        source: "processed".to_string(),
                        size: processed_batch.records.len(),
                        records: processed_batch
                            .records
                            .into_iter()
                            .filter_map(|r| {
                                r.transformed_data.map(|d| TelemetryRecord {
                                    id: r.original_id,
                                    timestamp: Utc::now(),
                                    record_type: match &d {
                                        TelemetryData::Metric(_) => TelemetryType::Metric,
                                        TelemetryData::Trace(_) => TelemetryType::Trace,
                                        TelemetryData::Log(_) => TelemetryType::Log,
                                        TelemetryData::Event(_) => TelemetryType::Event,
                                    },
                                    data: d,
                                    attributes: r.metadata,
                                    tags: HashMap::new(),
                                    resource: None,
                                    service: None,
                                })
                            })
                            .collect(),
                        metadata: processed_batch.metadata,
                    };
                }
                Err(e) => {
                    error!("Processor {} failed: {}", processor.name(), e);
                    return Err(e);
                }
            }

            let processing_time = start_time.elapsed();
            info!(
                "Processor {} completed in {:?}",
                processor.name(),
                processing_time
            );
        }

        Ok(ProcessedBatch {
            original_batch_id: current_batch.id,
            timestamp: Utc::now(),
            status: bridge_core::types::ProcessingStatus::Success,
            records: current_batch
                .records
                .into_iter()
                .map(|r| ProcessedRecord {
                    original_id: r.id,
                    status: bridge_core::types::ProcessingStatus::Success,
                    transformed_data: Some(r.data),
                    metadata: r.attributes,
                    errors: vec![],
                })
                .collect(),
            metadata: current_batch.metadata,
            errors: vec![],
        })
    }

    /// Start all processors
    pub async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting processor pipeline");

        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        for processor in &mut self.processors {
            info!("Starting processor: {}", processor.name());
            if let Err(e) = processor.health_check().await {
                error!("Failed to start processor {}: {}", processor.name(), e);
                return Err(e);
            }
        }

        info!("Processor pipeline started");
        Ok(())
    }

    /// Stop all processors
    pub async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping processor pipeline");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        for processor in &mut self.processors {
            info!("Stopping processor: {}", processor.name());
            if let Err(e) = processor.shutdown().await {
                error!("Failed to stop processor {}: {}", processor.name(), e);
                return Err(e);
            }
        }

        info!("Processor pipeline stopped");
        Ok(())
    }

    /// Check if pipeline is running
    pub async fn is_running(&self) -> bool {
        let is_running = self.is_running.read().await;
        *is_running
    }

    /// Get processor statistics
    pub async fn get_stats(&self) -> BridgeResult<HashMap<String, ProcessorStats>> {
        let mut stats = HashMap::new();

        for processor in &self.processors {
            match processor.get_stats().await {
                Ok(processor_stats) => {
                    stats.insert(processor.name().to_string(), processor_stats);
                }
                Err(e) => {
                    error!(
                        "Failed to get stats for processor {}: {}",
                        processor.name(),
                        e
                    );
                    return Err(e);
                }
            }
        }

        Ok(stats)
    }
}
