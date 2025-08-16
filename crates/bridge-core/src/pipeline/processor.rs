//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Pipeline processor for the OpenTelemetry Data Lake Bridge
//!
//! This module provides the main pipeline processing logic for orchestrating
//! telemetry data flow through receivers, processors, and exporters.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::error::{BridgeError, BridgeResult};
use crate::traits::{LakehouseExporter, TelemetryProcessor, TelemetryReceiver};
use crate::types::{ExportResult, ProcessedBatch, TelemetryBatch};

use super::config::PipelineConfig;
use super::state::{PipelineState, PipelineStats};

/// Telemetry ingestion pipeline
pub struct TelemetryIngestionPipeline {
    /// Pipeline configuration
    config: PipelineConfig,

    /// Telemetry receivers
    receivers: Vec<Arc<dyn TelemetryReceiver>>,

    /// Telemetry processors
    processors: Vec<Arc<dyn TelemetryProcessor>>,

    /// Lakehouse exporters
    exporters: Vec<Arc<dyn LakehouseExporter>>,

    /// Pipeline state
    state: Arc<RwLock<PipelineState>>,

    /// Pipeline statistics
    stats: Arc<RwLock<PipelineStats>>,

    /// Shutdown signal
    shutdown_tx: broadcast::Sender<()>,
}

impl TelemetryIngestionPipeline {
    /// Create a new telemetry ingestion pipeline
    pub fn new(config: PipelineConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            config,
            receivers: Vec::new(),
            processors: Vec::new(),
            exporters: Vec::new(),
            state: Arc::new(RwLock::new(PipelineState::default())),
            stats: Arc::new(RwLock::new(PipelineStats::default())),
            shutdown_tx,
        }
    }

    /// Add a telemetry receiver to the pipeline
    pub fn add_receiver(&mut self, receiver: Arc<dyn TelemetryReceiver>) {
        self.receivers.push(receiver);
        info!("Added receiver to pipeline: {}", self.config.name);
    }

    /// Add a telemetry processor to the pipeline
    pub fn add_processor(&mut self, processor: Arc<dyn TelemetryProcessor>) {
        self.processors.push(processor);
        info!("Added processor to pipeline: {}", self.config.name);
    }

    /// Add a lakehouse exporter to the pipeline
    pub fn add_exporter(&mut self, exporter: Arc<dyn LakehouseExporter>) {
        self.exporters.push(exporter);
        info!("Added exporter to pipeline: {}", self.config.name);
    }

    /// Start the pipeline
    pub async fn start(&mut self) -> BridgeResult<()> {
        let mut state = self.state.write().await;
        if state.running {
            return Err(BridgeError::internal("Pipeline is already running"));
        }

        state.running = true;
        state.healthy = true;
        state.start_time = Some(chrono::Utc::now());
        state.error_count = 0;
        state.last_error_time = None;
        state.last_error_message = None;

        info!(
            "Starting telemetry ingestion pipeline: {}",
            self.config.name
        );

        // Validate pipeline components
        self.validate_pipeline().await?;

        // Start pipeline tasks
        self.start_pipeline_tasks().await?;

        info!("Telemetry ingestion pipeline started: {}", self.config.name);
        Ok(())
    }

    /// Stop the pipeline
    pub async fn stop(&mut self) -> BridgeResult<()> {
        let mut state = self.state.write().await;
        if !state.running {
            return Err(BridgeError::internal("Pipeline is not running"));
        }

        info!(
            "Stopping telemetry ingestion pipeline: {}",
            self.config.name
        );

        // Send shutdown signal
        if let Err(e) = self.shutdown_tx.send(()) {
            warn!("Failed to send shutdown signal: {}", e);
        }

        state.running = false;
        state.stop_time = Some(chrono::Utc::now());

        // Shutdown components
        self.shutdown_components().await?;

        info!("Telemetry ingestion pipeline stopped: {}", self.config.name);
        Ok(())
    }

    /// Get pipeline state
    pub async fn get_state(&self) -> PipelineState {
        self.state.read().await.clone()
    }

    /// Get pipeline statistics
    pub async fn get_stats(&self) -> PipelineStats {
        self.stats.read().await.clone()
    }

    /// Check if pipeline is healthy
    pub async fn health_check(&self) -> BridgeResult<bool> {
        let state = self.state.read().await;
        Ok(state.running && state.healthy)
    }

    /// Process a single telemetry batch through the pipeline
    pub async fn process_batch(&self, batch: TelemetryBatch) -> BridgeResult<ExportResult> {
        let start_time = Instant::now();
        let batch_id = batch.id;

        debug!(
            "Processing batch {} with {} records",
            batch_id,
            batch.records.len()
        );

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_batches += 1;
            stats.total_records += batch.records.len() as u64;
            stats.last_process_time = Some(chrono::Utc::now());
        }

        // Process through processors
        let processed_batch = self.process_through_processors(batch).await?;

        // Export through exporters
        let export_result = self.export_through_exporters(processed_batch).await?;

        // Update processing time statistics
        let processing_time = start_time.elapsed();
        {
            let mut stats = self.stats.write().await;
            stats.total_processing_time_ms += processing_time.as_millis() as u64;
            stats.avg_processing_time_ms =
                stats.total_processing_time_ms as f64 / stats.total_batches as f64;
        }

        debug!(
            "Completed processing batch {} in {:?}",
            batch_id, processing_time
        );
        Ok(export_result)
    }

    /// Validate pipeline components
    async fn validate_pipeline(&self) -> BridgeResult<()> {
        if self.receivers.is_empty() {
            return Err(BridgeError::configuration("No receivers configured"));
        }

        if self.exporters.is_empty() {
            return Err(BridgeError::configuration("No exporters configured"));
        }

        // Health check all components
        for (i, receiver) in self.receivers.iter().enumerate() {
            if !receiver.health_check().await? {
                return Err(BridgeError::internal(format!(
                    "Receiver {} is not healthy",
                    i
                )));
            }
        }

        for (i, processor) in self.processors.iter().enumerate() {
            if !processor.health_check().await? {
                return Err(BridgeError::internal(format!(
                    "Processor {} is not healthy",
                    i
                )));
            }
        }

        for (i, exporter) in self.exporters.iter().enumerate() {
            if !exporter.health_check().await? {
                return Err(BridgeError::internal(format!(
                    "Exporter {} is not healthy",
                    i
                )));
            }
        }

        Ok(())
    }

    /// Start pipeline tasks
    async fn start_pipeline_tasks(&self) -> BridgeResult<()> {
        let config = self.config.clone();
        let state = Arc::clone(&self.state);
        let stats = Arc::clone(&self.stats);
        let receivers = self.receivers.clone();
        let processors = self.processors.clone();
        let exporters = self.exporters.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        // Start main processing task
        tokio::spawn(async move {
            let mut flush_interval = interval(Duration::from_millis(config.flush_interval_ms));

            loop {
                tokio::select! {
                    _ = flush_interval.tick() => {
                        if let Err(e) = Self::process_pending_data(
                            &receivers,
                            &processors,
                            &exporters,
                            &state,
                            &stats,
                            &config,
                        ).await {
                            error!("Error processing pending data: {}", e);
                            Self::update_pipeline_error(&state, &e).await;
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Received shutdown signal, stopping pipeline tasks");
                        break;
                    }
                }
            }
        });

        // Start health check task if enabled
        if self.config.enable_health_checks {
            let config = self.config.clone();
            let state = Arc::clone(&self.state);
            let receivers = self.receivers.clone();
            let processors = self.processors.clone();
            let exporters = self.exporters.clone();
            let mut shutdown_rx = self.shutdown_tx.subscribe();

            tokio::spawn(async move {
                let mut health_interval =
                    interval(Duration::from_millis(config.health_check_interval_ms));

                loop {
                    tokio::select! {
                        _ = health_interval.tick() => {
                            if let Err(e) = Self::perform_health_checks(
                                &receivers,
                                &processors,
                                &exporters,
                                &state,
                            ).await {
                                error!("Error during health checks: {}", e);
                                Self::update_pipeline_error(&state, &e).await;
                            }
                        }
                        _ = shutdown_rx.recv() => {
                            info!("Received shutdown signal, stopping health check task");
                            break;
                        }
                    }
                }
            });
        }

        Ok(())
    }

    /// Process pending data from receivers
    async fn process_pending_data(
        receivers: &[Arc<dyn TelemetryReceiver>],
        processors: &[Arc<dyn TelemetryProcessor>],
        exporters: &[Arc<dyn LakehouseExporter>],
        state: &Arc<RwLock<PipelineState>>,
        stats: &Arc<RwLock<PipelineStats>>,
        config: &PipelineConfig,
    ) -> BridgeResult<()> {
        let mut total_processed = 0;

        for receiver in receivers {
            // Check backpressure
            if config.enable_backpressure {
                let receiver_stats = receiver.get_stats().await?;
                let buffer_usage =
                    (receiver_stats.total_records as f64 / config.buffer_size as f64) * 100.0;

                if buffer_usage > config.backpressure_threshold as f64 {
                    warn!("Backpressure threshold exceeded: {:.2}%", buffer_usage);
                    continue;
                }
            }

            // Receive data from receiver
            match receiver.receive().await {
                Ok(batch) => {
                    if batch.records.is_empty() {
                        continue;
                    }

                    // Process through processors
                    let processed_batch =
                        Self::process_through_processors_internal(batch, processors).await?;

                    // Export through exporters
                    Self::export_through_exporters_internal(processed_batch.clone(), exporters)
                        .await?;

                    total_processed += 1;

                    // Update statistics
                    Self::update_processing_stats(stats, &processed_batch).await;
                }
                Err(e) => {
                    error!("Error receiving data from receiver: {}", e);
                    Self::update_pipeline_error(state, &e).await;
                }
            }
        }

        if total_processed > 0 {
            debug!("Processed {} batches in this cycle", total_processed);
        }

        Ok(())
    }

    /// Perform health checks on all components
    async fn perform_health_checks(
        receivers: &[Arc<dyn TelemetryReceiver>],
        processors: &[Arc<dyn TelemetryProcessor>],
        exporters: &[Arc<dyn LakehouseExporter>],
        state: &Arc<RwLock<PipelineState>>,
    ) -> BridgeResult<()> {
        let mut all_healthy = true;

        // Check receivers
        for (i, receiver) in receivers.iter().enumerate() {
            if !receiver.health_check().await? {
                error!("Receiver {} is not healthy", i);
                all_healthy = false;
            }
        }

        // Check processors
        for (i, processor) in processors.iter().enumerate() {
            if !processor.health_check().await? {
                error!("Processor {} is not healthy", i);
                all_healthy = false;
            }
        }

        // Check exporters
        for (i, exporter) in exporters.iter().enumerate() {
            if !exporter.health_check().await? {
                error!("Exporter {} is not healthy", i);
                all_healthy = false;
            }
        }

        // Update pipeline state
        {
            let mut state = state.write().await;
            state.healthy = all_healthy;
        }

        Ok(())
    }

    /// Process batch through all processors
    async fn process_through_processors(
        &self,
        batch: TelemetryBatch,
    ) -> BridgeResult<ProcessedBatch> {
        Self::process_through_processors_internal(batch, &self.processors).await
    }

    /// Process batch through all processors (internal)
    async fn process_through_processors_internal(
        batch: TelemetryBatch,
        processors: &[Arc<dyn TelemetryProcessor>],
    ) -> BridgeResult<ProcessedBatch> {
        let mut current_batch = batch;

        for (i, processor) in processors.iter().enumerate() {
            match processor.process(current_batch).await {
                Ok(processed_batch) => {
                    debug!("Processor {} processed batch successfully", i);
                    current_batch = TelemetryBatch {
                        id: processed_batch.original_batch_id,
                        timestamp: processed_batch.timestamp,
                        source: "processed".to_string(),
                        size: processed_batch.records.len(),
                        records: processed_batch
                            .records
                            .into_iter()
                            .filter_map(|r| {
                                r.transformed_data.map(|d| {
                                    crate::types::TelemetryRecord::new(
                                        crate::types::TelemetryType::Metric, // This should be determined from the data
                                        d,
                                    )
                                })
                            })
                            .collect(),
                        metadata: processed_batch.metadata,
                    };
                }
                Err(e) => {
                    error!("Processor {} failed to process batch: {}", i, e);
                    return Err(e);
                }
            }
        }

        // Convert back to ProcessedBatch
        Ok(ProcessedBatch {
            original_batch_id: current_batch.id,
            timestamp: current_batch.timestamp,
            status: crate::types::ProcessingStatus::Success,
            records: current_batch
                .records
                .into_iter()
                .map(|r| crate::types::ProcessedRecord {
                    original_id: r.id,
                    status: crate::types::ProcessingStatus::Success,
                    transformed_data: Some(r.data),
                    metadata: r.attributes,
                    errors: vec![],
                })
                .collect(),
            metadata: current_batch.metadata,
            errors: vec![],
        })
    }

    /// Export batch through all exporters
    async fn export_through_exporters(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult> {
        Self::export_through_exporters_internal(batch, &self.exporters).await
    }

    /// Export batch through all exporters (internal)
    async fn export_through_exporters_internal(
        batch: ProcessedBatch,
        exporters: &[Arc<dyn LakehouseExporter>],
    ) -> BridgeResult<ExportResult> {
        let mut last_result = None;

        for (i, exporter) in exporters.iter().enumerate() {
            match exporter.export(batch.clone()).await {
                Ok(result) => {
                    debug!("Exporter {} exported batch successfully", i);
                    last_result = Some(result);
                }
                Err(e) => {
                    error!("Exporter {} failed to export batch: {}", i, e);
                    // Continue with other exporters
                }
            }
        }

        last_result.ok_or_else(|| BridgeError::export("All exporters failed to export batch"))
    }

    /// Update processing statistics
    async fn update_processing_stats(stats: &Arc<RwLock<PipelineStats>>, batch: &ProcessedBatch) {
        let mut stats = stats.write().await;
        stats.total_batches += 1;
        stats.total_records += batch.records.len() as u64;
        stats.last_process_time = Some(chrono::Utc::now());
    }

    /// Update pipeline error state
    async fn update_pipeline_error(state: &Arc<RwLock<PipelineState>>, error: &BridgeError) {
        let mut state = state.write().await;
        state.error_count += 1;
        state.last_error_time = Some(chrono::Utc::now());
        state.last_error_message = Some(error.to_string());
        state.healthy = false;
    }

    /// Shutdown all components
    async fn shutdown_components(&self) -> BridgeResult<()> {
        // Shutdown receivers
        for (i, receiver) in self.receivers.iter().enumerate() {
            if let Err(e) = receiver.shutdown().await {
                error!("Error shutting down receiver {}: {}", i, e);
            }
        }

        // Shutdown processors
        for (i, processor) in self.processors.iter().enumerate() {
            if let Err(e) = processor.shutdown().await {
                error!("Error shutting down processor {}: {}", i, e);
            }
        }

        // Shutdown exporters
        for (i, exporter) in self.exporters.iter().enumerate() {
            if let Err(e) = exporter.shutdown().await {
                error!("Error shutting down exporter {}: {}", i, e);
            }
        }

        Ok(())
    }
}

impl Drop for TelemetryIngestionPipeline {
    fn drop(&mut self) {
        // Ensure shutdown signal is sent
        let _ = self.shutdown_tx.send(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{ExporterStats, ProcessorStats, ReceiverStats};
    use crate::types::{
        MetricData, MetricType, MetricValue, TelemetryData, TelemetryRecord, TelemetryType,
    };
    use async_trait::async_trait;
    use mockall::predicate::*;
    use mockall::*;

    mock! {
        TestReceiver {}

        #[async_trait]
        impl TelemetryReceiver for TestReceiver {
            async fn receive(&self) -> BridgeResult<TelemetryBatch>;
            async fn health_check(&self) -> BridgeResult<bool>;
            async fn get_stats(&self) -> BridgeResult<ReceiverStats>;
            async fn shutdown(&self) -> BridgeResult<()>;
        }
    }

    mock! {
        TestProcessor {}

        #[async_trait]
        impl TelemetryProcessor for TestProcessor {
            async fn process(&self, batch: TelemetryBatch) -> BridgeResult<ProcessedBatch>;
            fn name(&self) -> &str;
            fn version(&self) -> &str;
            async fn health_check(&self) -> BridgeResult<bool>;
            async fn get_stats(&self) -> BridgeResult<ProcessorStats>;
            async fn shutdown(&self) -> BridgeResult<()>;
        }
    }

    mock! {
        TestExporter {}

        #[async_trait]
        impl LakehouseExporter for TestExporter {
            async fn export(&self, batch: ProcessedBatch) -> BridgeResult<ExportResult>;
            fn name(&self) -> &str;
            fn version(&self) -> &str;
            async fn health_check(&self) -> BridgeResult<bool>;
            async fn get_stats(&self) -> BridgeResult<ExporterStats>;
            async fn shutdown(&self) -> BridgeResult<()>;
        }
    }

    #[tokio::test]
    async fn test_pipeline_creation() {
        let config = PipelineConfig::default();
        let pipeline = TelemetryIngestionPipeline::new(config);

        let state = pipeline.get_state().await;
        assert!(!state.running);
        assert!(state.healthy);
    }

    #[tokio::test]
    async fn test_pipeline_validation() {
        let config = PipelineConfig::default();
        let mut pipeline = TelemetryIngestionPipeline::new(config);

        // Add a receiver
        let mut mock_receiver = MockTestReceiver::new();
        mock_receiver
            .expect_health_check()
            .times(1)
            .returning(|| Ok(true));
        pipeline.add_receiver(Arc::new(mock_receiver));

        // Add an exporter
        let mut mock_exporter = MockTestExporter::new();
        mock_exporter
            .expect_health_check()
            .times(1)
            .returning(|| Ok(true));
        pipeline.add_exporter(Arc::new(mock_exporter));

        // Validation should pass
        let result = pipeline.validate_pipeline().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pipeline_validation_no_receivers() {
        let config = PipelineConfig::default();
        let pipeline = TelemetryIngestionPipeline::new(config);

        // Validation should fail without receivers
        let result = pipeline.validate_pipeline().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pipeline_validation_no_exporters() {
        let config = PipelineConfig::default();
        let mut pipeline = TelemetryIngestionPipeline::new(config);

        // Add only a receiver (no health check expectation since validation fails early)
        let mock_receiver = MockTestReceiver::new();
        pipeline.add_receiver(Arc::new(mock_receiver));

        // Validation should fail without exporters
        let result = pipeline.validate_pipeline().await;
        assert!(result.is_err());
    }
}
