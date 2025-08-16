//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Minimal OTel-Arrow Protocol (OTAP) Example
//!
//! This example demonstrates the basic OTAP protocol integration without
//! external connector dependencies.

use bridge_core::BridgeResult;
use ingestion::{
    exporters::batch_exporter::BatchExporter, exporters::ExporterPipeline,
    processors::batch_processor::BatchProcessor, processors::ProcessorPipeline,
    receivers::otap_receiver::OtapReceiver, receivers::ReceiverManager,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Minimal OTAP pipeline configuration
#[derive(Debug, Clone)]
struct MinimalOtapPipelineConfig {
    name: String,
    max_batch_size: usize,
    flush_interval_ms: u64,
    buffer_size: usize,
    compression_level: u32,
}

impl Default for MinimalOtapPipelineConfig {
    fn default() -> Self {
        Self {
            name: "minimal-otap-pipeline".to_string(),
            max_batch_size: 1000,
            flush_interval_ms: 5000,
            buffer_size: 10000,
            compression_level: 6,
        }
    }
}

/// Minimal OTAP telemetry ingestion pipeline
pub struct MinimalOtapPipeline {
    config: MinimalOtapPipelineConfig,
    is_running: Arc<RwLock<bool>>,
    receiver_manager: ReceiverManager,
    processor_pipeline: ProcessorPipeline,
    exporter_pipeline: ExporterPipeline,
}

impl MinimalOtapPipeline {
    /// Create a new minimal OTAP pipeline
    pub fn new(config: MinimalOtapPipelineConfig) -> Self {
        info!("Creating minimal OTAP pipeline: {}", config.name);

        Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            receiver_manager: ReceiverManager::new(),
            processor_pipeline: ProcessorPipeline::new(),
            exporter_pipeline: ExporterPipeline::new(),
        }
    }

    /// Add OTAP receiver to the pipeline
    pub async fn add_otap_receiver(
        &mut self,
        name: String,
        endpoint: String,
        port: u16,
    ) -> BridgeResult<()> {
        info!("Adding OTAP receiver: {} on {}:{}", name, endpoint, port);

        let config = ingestion::receivers::otap_receiver::OtapReceiverConfig::new(endpoint, port);

        let receiver = OtapReceiver::new(&config).await?;
        self.receiver_manager.add_receiver(name, Box::new(receiver));
        Ok(())
    }

    /// Add batch processor to the pipeline
    pub async fn add_batch_processor(&mut self, name: String) -> BridgeResult<()> {
        info!("Adding batch processor: {}", name);

        let config = ingestion::processors::batch_processor::BatchProcessorConfig::new(
            self.config.max_batch_size,
            self.config.flush_interval_ms,
        );

        let processor = BatchProcessor::new(&config).await?;
        self.processor_pipeline.add_processor(Box::new(processor));
        Ok(())
    }

    /// Add batch exporter to the pipeline
    pub async fn add_batch_exporter(
        &mut self,
        name: String,
        destination: String,
    ) -> BridgeResult<()> {
        info!("Adding batch exporter: {} to {}", name, destination);

        let config = ingestion::exporters::batch_exporter::BatchExporterConfig::new(
            destination,
            self.config.max_batch_size,
        );

        let exporter = BatchExporter::new(&config).await?;
        self.exporter_pipeline.add_exporter(Box::new(exporter));
        Ok(())
    }

    /// Start the pipeline
    pub async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting minimal OTAP telemetry ingestion pipeline");

        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        // Start receiver manager
        self.receiver_manager.start_all().await?;

        // Start processor pipeline
        self.processor_pipeline.start().await?;

        // Start exporter pipeline
        self.exporter_pipeline.start().await?;

        info!("Minimal OTAP telemetry ingestion pipeline started successfully");
        Ok(())
    }

    /// Stop the pipeline
    pub async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping minimal OTAP telemetry ingestion pipeline");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        // Stop exporter pipeline
        self.exporter_pipeline.stop().await?;

        // Stop processor pipeline
        self.processor_pipeline.stop().await?;

        // Stop receiver manager
        self.receiver_manager.stop_all().await?;

        info!("Minimal OTAP telemetry ingestion pipeline stopped successfully");
        Ok(())
    }

    /// Check if pipeline is running
    pub async fn is_running(&self) -> bool {
        let is_running = self.is_running.read().await;
        *is_running
    }

    /// Get pipeline statistics
    pub async fn get_stats(&self) -> BridgeResult<()> {
        info!("Minimal OTAP Pipeline Statistics:");

        // Get receiver statistics
        let receiver_stats = self.receiver_manager.get_stats().await?;
        info!("Receiver Stats: {:?}", receiver_stats);

        // Get processor statistics
        let processor_stats = self.processor_pipeline.get_stats().await?;
        info!("Processor Stats: {:?}", processor_stats);

        // Note: ExporterPipeline doesn't have get_stats method yet
        info!("Exporter Stats: Not available");

        Ok(())
    }
}

/// Main function demonstrating minimal OTAP usage
#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting minimal OTAP example");

    // Create pipeline configuration
    let config = MinimalOtapPipelineConfig::default();
    let mut pipeline = MinimalOtapPipeline::new(config);

    // Add OTAP receiver
    pipeline
        .add_otap_receiver("otap-receiver".to_string(), "127.0.0.1".to_string(), 4319)
        .await?;

    // Add batch processor
    pipeline
        .add_batch_processor("batch-processor".to_string())
        .await?;

    // Add batch exporter (to memory for this example)
    pipeline
        .add_batch_exporter(
            "memory-exporter".to_string(),
            "memory://telemetry-data".to_string(),
        )
        .await?;

    // Start the pipeline
    pipeline.start().await?;

    // Let it run for a few seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Get statistics
    pipeline.get_stats().await?;

    // Stop the pipeline
    pipeline.stop().await?;

    info!("Minimal OTAP example completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_minimal_otap_pipeline_creation() {
        let config = MinimalOtapPipelineConfig::default();
        let pipeline = MinimalOtapPipeline::new(config);
        assert!(!pipeline.is_running().await);
    }

    #[tokio::test]
    async fn test_minimal_otap_pipeline_lifecycle() {
        let config = MinimalOtapPipelineConfig::default();
        let mut pipeline = MinimalOtapPipeline::new(config);

        // Test start
        let result = pipeline.start().await;
        assert!(result.is_ok());

        // Test running state
        assert!(pipeline.is_running().await);

        // Test stop
        let result = pipeline.stop().await;
        assert!(result.is_ok());

        // Test stopped state
        assert!(!pipeline.is_running().await);
    }
}
