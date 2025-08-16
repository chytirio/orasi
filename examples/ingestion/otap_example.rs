//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OTel-Arrow Protocol (OTAP) Example
//!
//! This example demonstrates how to use the OTAP protocol for telemetry ingestion
//! with significant compression improvements over standard OTLP.

use bridge_core::BridgeResult;
use ingestion::{
    OtapReceiver, OtapReceiverConfig, ReceiverFactory, ReceiverManager,
    BatchProcessor, BatchProcessorConfig, ProcessorFactory, ProcessorPipeline,
    BatchExporter, BatchExporterConfig, ExporterFactory, ExporterPipeline,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Pipeline configuration for OTAP example
#[derive(Debug, Clone)]
struct OtapPipelineConfig {
    name: String,
    max_batch_size: usize,
    flush_interval_ms: u64,
    buffer_size: usize,
    enable_backpressure: bool,
    backpressure_threshold: u8,
    enable_metrics: bool,
    enable_health_checks: bool,
    health_check_interval_ms: u64,
}

impl Default for OtapPipelineConfig {
    fn default() -> Self {
        Self {
            name: "otap-pipeline".to_string(),
            max_batch_size: 1000,
            flush_interval_ms: 5000,
            buffer_size: 10000,
            enable_backpressure: true,
            backpressure_threshold: 80,
            enable_metrics: true,
            enable_health_checks: true,
            health_check_interval_ms: 30000,
        }
    }
}

/// OTAP telemetry ingestion pipeline
struct OtapTelemetryIngestionPipeline {
    config: OtapPipelineConfig,
    receiver_manager: ReceiverManager,
    processor_pipeline: ProcessorPipeline,
    exporter_pipeline: ExporterPipeline,
    is_running: Arc<RwLock<bool>>,
}

impl OtapTelemetryIngestionPipeline {
    /// Create new OTAP pipeline
    pub fn new(config: OtapPipelineConfig) -> Self {
        Self {
            receiver_manager: ReceiverManager::new(),
            processor_pipeline: ProcessorPipeline::new(),
            exporter_pipeline: ExporterPipeline::new(),
            is_running: Arc::new(RwLock::new(false)),
            config,
        }
    }

    /// Add OTAP receiver to the pipeline
    pub fn add_otap_receiver(&mut self, name: String, endpoint: String, port: u16) -> BridgeResult<()> {
        info!("Adding OTAP receiver: {} on {}:{}", name, endpoint, port);

        let config = OtapReceiverConfig::new(endpoint, port);
        let receiver = OtapReceiver::new(&config)?;
        let receiver = Arc::new(receiver);

        self.receiver_manager.add_receiver(name, receiver);
        Ok(())
    }

    /// Add batch processor to the pipeline
    pub fn add_batch_processor(&mut self, name: String) -> BridgeResult<()> {
        info!("Adding batch processor: {}", name);

        let config = BatchProcessorConfig::new(
            self.config.max_batch_size,
            self.config.flush_interval_ms,
        );
        let processor = BatchProcessor::new(&config)?;
        let processor = Arc::new(processor);

        self.processor_pipeline.add_processor(name, processor);
        Ok(())
    }

    /// Add batch exporter to the pipeline
    pub fn add_batch_exporter(&mut self, name: String, destination: String) -> BridgeResult<()> {
        info!("Adding batch exporter: {} to {}", name, destination);

        let config = BatchExporterConfig::new(destination);
        let exporter = BatchExporter::new(&config)?;
        let exporter = Arc::new(exporter);

        self.exporter_pipeline.add_exporter(name, exporter);
        Ok(())
    }

    /// Start the pipeline
    pub async fn start(&mut self) -> BridgeResult<()> {
        info!("Starting OTAP telemetry ingestion pipeline");

        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        // Start receiver manager
        self.receiver_manager.start().await?;

        // Start processor pipeline
        self.processor_pipeline.start().await?;

        // Start exporter pipeline
        self.exporter_pipeline.start().await?;

        info!("OTAP telemetry ingestion pipeline started successfully");
        Ok(())
    }

    /// Stop the pipeline
    pub async fn stop(&mut self) -> BridgeResult<()> {
        info!("Stopping OTAP telemetry ingestion pipeline");

        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        // Stop exporter pipeline
        self.exporter_pipeline.stop().await?;

        // Stop processor pipeline
        self.processor_pipeline.stop().await?;

        // Stop receiver manager
        self.receiver_manager.stop().await?;

        info!("OTAP telemetry ingestion pipeline stopped successfully");
        Ok(())
    }

    /// Check if pipeline is running
    pub async fn is_running(&self) -> bool {
        let is_running = self.is_running.read().await;
        *is_running
    }

    /// Get pipeline statistics
    pub async fn get_stats(&self) -> BridgeResult<()> {
        info!("OTAP Pipeline Statistics:");

        // Get receiver statistics
        let receiver_stats = self.receiver_manager.get_stats().await?;
        info!("Receiver Stats: {:?}", receiver_stats);

        // Get processor statistics
        let processor_stats = self.processor_pipeline.get_stats().await?;
        info!("Processor Stats: {:?}", processor_stats);

        // Get exporter statistics
        let exporter_stats = self.exporter_pipeline.get_stats().await?;
        info!("Exporter Stats: {:?}", exporter_stats);

        Ok(())
    }
}

/// Main function demonstrating OTAP usage
#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting OTAP example for OTel-Arrow Protocol ingestion");

    // Create pipeline configuration
    let config = OtapPipelineConfig::default();

    // Create pipeline
    let mut pipeline = OtapTelemetryIngestionPipeline::new(config);

    // Add OTAP receiver
    pipeline.add_otap_receiver(
        "otap-receiver".to_string(),
        "0.0.0.0".to_string(),
        4319,
    )?;

    // Add batch processor
    pipeline.add_batch_processor("batch-processor".to_string())?;

    // Add batch exporter (example: export to file)
    pipeline.add_batch_exporter(
        "file-exporter".to_string(),
        "file:///tmp/otap-telemetry.json".to_string(),
    )?;

    // Start pipeline
    pipeline.start().await?;

    info!("OTAP pipeline started successfully");

    // Run the pipeline for a few seconds
    let pipeline_handle = tokio::spawn(async move {
        // Simulate running for 30 seconds
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

        // Get statistics
        if let Err(e) = pipeline.get_stats().await {
            warn!("Failed to get pipeline statistics: {}", e);
        }

        // Stop pipeline
        if let Err(e) = pipeline.stop().await {
            warn!("Failed to stop pipeline: {}", e);
        }
    });

    // Wait for pipeline to complete
    if let Err(e) = pipeline_handle.await {
        warn!("Pipeline handle error: {}", e);
    }

    info!("OTAP example completed successfully");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_otap_pipeline_creation() {
        let config = OtapPipelineConfig::default();
        let mut pipeline = OtapTelemetryIngestionPipeline::new(config);

        // Test adding OTAP receiver
        let result = pipeline.add_otap_receiver(
            "test-receiver".to_string(),
            "127.0.0.1".to_string(),
            4319,
        );
        assert!(result.is_ok());

        // Test adding batch processor
        let result = pipeline.add_batch_processor("test-processor".to_string());
        assert!(result.is_ok());

        // Test adding batch exporter
        let result = pipeline.add_batch_exporter(
            "test-exporter".to_string(),
            "file:///tmp/test.json".to_string(),
        );
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_otap_pipeline_lifecycle() {
        let config = OtapPipelineConfig::default();
        let mut pipeline = OtapTelemetryIngestionPipeline::new(config);

        // Add components
        pipeline.add_otap_receiver("test-receiver".to_string(), "127.0.0.1".to_string(), 4319).unwrap();
        pipeline.add_batch_processor("test-processor".to_string()).unwrap();
        pipeline.add_batch_exporter("test-exporter".to_string(), "file:///tmp/test.json".to_string()).unwrap();

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
