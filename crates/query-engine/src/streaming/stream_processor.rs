//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Stream processor for streaming queries
//!
//! This module provides processing capabilities for streaming queries.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{
    StreamingQueryConfig, StreamingQueryResult, StreamingQueryRow, StreamingQueryStats,
    StreamingQueryValue, WindowInfo, WindowType,
};

/// Stream processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamProcessorConfig {
    /// Processor name
    pub name: String,

    /// Processor version
    pub version: String,

    /// Enable processing
    pub enable_processing: bool,

    /// Processing batch size
    pub batch_size: usize,

    /// Processing interval in milliseconds
    pub processing_interval_ms: u64,

    /// Enable windowing
    pub enable_windowing: bool,

    /// Default window size in milliseconds
    pub default_window_size_ms: u64,

    /// Enable aggregation
    pub enable_aggregation: bool,

    /// Enable filtering
    pub enable_filtering: bool,

    /// Enable transformation
    pub enable_transformation: bool,

    /// Maximum processing time in milliseconds
    pub max_processing_time_ms: u64,

    /// Enable backpressure handling
    pub enable_backpressure: bool,

    /// Backpressure threshold (percentage)
    pub backpressure_threshold: u8,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl Default for StreamProcessorConfig {
    fn default() -> Self {
        Self {
            name: "default_stream_processor".to_string(),
            version: "1.0.0".to_string(),
            enable_processing: true,
            batch_size: 1000,
            processing_interval_ms: 100,
            enable_windowing: true,
            default_window_size_ms: 60000, // 1 minute
            enable_aggregation: true,
            enable_filtering: true,
            enable_transformation: true,
            max_processing_time_ms: 5000,
            enable_backpressure: true,
            backpressure_threshold: 80,
            additional_config: HashMap::new(),
        }
    }
}

/// Stream processing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamProcessingStats {
    /// Total records processed
    pub total_records_processed: u64,

    /// Records processed per second
    pub records_per_second: f64,

    /// Average processing time in milliseconds
    pub avg_processing_time_ms: f64,

    /// Total processing time in milliseconds
    pub total_processing_time_ms: u64,

    /// Error count
    pub error_count: u64,

    /// Last processing time
    pub last_processing_time: Option<DateTime<Utc>>,

    /// Active windows
    pub active_windows: u64,

    /// Completed windows
    pub completed_windows: u64,
}

/// Stream processing context
#[derive(Debug, Clone)]
pub struct StreamProcessingContext {
    /// Processing timestamp
    pub timestamp: DateTime<Utc>,

    /// Window information
    pub window_info: Option<WindowInfo>,

    /// Processing metadata
    pub metadata: HashMap<String, String>,
}

/// Stream processing result
#[derive(Debug, Clone)]
pub struct StreamProcessingResult {
    /// Processing timestamp
    pub timestamp: DateTime<Utc>,

    /// Processed data
    pub data: Vec<StreamingQueryRow>,

    /// Processing statistics
    pub stats: StreamProcessingStats,

    /// Processing metadata
    pub metadata: HashMap<String, String>,
}

/// Stream processor trait
#[async_trait]
pub trait StreamProcessor: Send + Sync {
    /// Process streaming data
    async fn process(&self, data: Vec<StreamingQueryRow>) -> BridgeResult<StreamProcessingResult>;

    /// Get processor statistics
    async fn get_stats(&self) -> BridgeResult<StreamProcessingStats>;

    /// Start processing
    async fn start(&mut self) -> BridgeResult<()>;

    /// Stop processing
    async fn stop(&mut self) -> BridgeResult<()>;

    /// Check if processor is running
    fn is_running(&self) -> bool;
}

/// Default stream processor implementation
pub struct DefaultStreamProcessor {
    config: StreamProcessorConfig,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<StreamProcessingStats>>,
    data_sender: Option<mpsc::Sender<Vec<StreamingQueryRow>>>,
    data_receiver: Option<mpsc::Receiver<Vec<StreamingQueryRow>>>,
    result_sender: Option<mpsc::Sender<StreamProcessingResult>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
    start_time: Option<DateTime<Utc>>,
}

impl DefaultStreamProcessor {
    /// Create a new stream processor
    pub fn new(config: StreamProcessorConfig) -> Self {
        Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(StreamProcessingStats {
                total_records_processed: 0,
                records_per_second: 0.0,
                avg_processing_time_ms: 0.0,
                total_processing_time_ms: 0,
                error_count: 0,
                last_processing_time: None,
                active_windows: 0,
                completed_windows: 0,
            })),
            data_sender: None,
            data_receiver: None,
            result_sender: None,
            task_handle: None,
            start_time: None,
        }
    }

    /// Initialize processing channels
    fn initialize_channels(&mut self) {
        let (data_tx, data_rx) = mpsc::channel(self.config.batch_size * 2);
        let (result_tx, _result_rx) = mpsc::channel(self.config.batch_size);

        self.data_sender = Some(data_tx);
        self.data_receiver = Some(data_rx);
        self.result_sender = Some(result_tx);
    }

    /// Process data with windowing
    async fn process_with_windowing(
        &self,
        data: &[StreamingQueryRow],
    ) -> BridgeResult<Vec<StreamingQueryRow>> {
        if !self.config.enable_windowing {
            return Ok(data.to_vec());
        }

        let mut windowed_data = Vec::new();
        let window_size = Duration::from_millis(self.config.default_window_size_ms);
        let now = Utc::now();

        for row in data.iter() {
            // Create window info
            let window_start =
                now - chrono::Duration::milliseconds(self.config.default_window_size_ms as i64);
            let window_info = WindowInfo {
                start_time: window_start,
                end_time: now,
                window_type: WindowType::Time,
                window_size_ms: self.config.default_window_size_ms,
            };

            // Add window metadata to row
            let mut row_data = row.data.clone();
            row_data.insert(
                "window_start".to_string(),
                StreamingQueryValue::Timestamp(window_start),
            );
            row_data.insert(
                "window_end".to_string(),
                StreamingQueryValue::Timestamp(now),
            );
            row_data.insert(
                "window_size_ms".to_string(),
                StreamingQueryValue::Integer(self.config.default_window_size_ms as i64),
            );

            let windowed_row = StreamingQueryRow {
                id: row.id,
                data: row_data,
                metadata: {
                    let mut metadata = row.metadata.clone();
                    metadata.insert("window_id".to_string(), Uuid::new_v4().to_string());
                    metadata.insert("window_type".to_string(), "time".to_string());
                    metadata
                },
                timestamp: row.timestamp,
            };

            windowed_data.push(windowed_row);
        }

        Ok(windowed_data)
    }

    /// Apply filtering to data
    async fn apply_filtering(
        &self,
        data: &[StreamingQueryRow],
    ) -> BridgeResult<Vec<StreamingQueryRow>> {
        if !self.config.enable_filtering {
            return Ok(data.to_vec());
        }

        let filtered_data: Vec<StreamingQueryRow> = data
            .iter()
            .cloned()
            .filter(|row| {
                // Basic filtering: exclude rows with null values in key fields
                !row.data
                    .values()
                    .any(|value| matches!(value, StreamingQueryValue::Null))
            })
            .collect();

        Ok(filtered_data)
    }

    /// Apply transformations to data
    async fn apply_transformations(
        &self,
        data: &[StreamingQueryRow],
    ) -> BridgeResult<Vec<StreamingQueryRow>> {
        if !self.config.enable_transformation {
            return Ok(data.to_vec());
        }

        let transformed_data: Vec<StreamingQueryRow> = data
            .iter()
            .cloned()
            .map(|row| {
                let mut transformed_data = row.data.clone();

                // Add processing timestamp
                transformed_data.insert(
                    "processed_at".to_string(),
                    StreamingQueryValue::Timestamp(Utc::now()),
                );

                // Add processor metadata
                transformed_data.insert(
                    "processor_name".to_string(),
                    StreamingQueryValue::String(self.config.name.clone()),
                );

                let mut transformed_metadata = row.metadata.clone();
                transformed_metadata.insert("transformed".to_string(), "true".to_string());
                transformed_metadata
                    .insert("processor_version".to_string(), self.config.version.clone());

                StreamingQueryRow {
                    id: row.id,
                    data: transformed_data,
                    metadata: transformed_metadata,
                    timestamp: row.timestamp,
                }
            })
            .collect();

        Ok(transformed_data)
    }

    /// Apply aggregation to data
    async fn apply_aggregation(
        &self,
        data: &[StreamingQueryRow],
    ) -> BridgeResult<Vec<StreamingQueryRow>> {
        if !self.config.enable_aggregation || data.is_empty() {
            return Ok(data.to_vec());
        }

        // Simple aggregation: group by timestamp (hour) and count records
        let mut aggregated_data = Vec::new();
        let mut hour_groups: HashMap<String, Vec<StreamingQueryRow>> = HashMap::new();

        for row in data.iter() {
            let hour_key = format!(
                "{}-{:02}-{:02}-{:02}",
                row.timestamp.year(),
                row.timestamp.month(),
                row.timestamp.day(),
                row.timestamp.hour()
            );

            hour_groups
                .entry(hour_key)
                .or_insert_with(Vec::new)
                .push(row.clone());
        }

        for (hour_key, rows) in hour_groups {
            let count = rows.len() as i64;
            let first_row = &rows[0];

            let mut aggregated_row_data = HashMap::new();
            aggregated_row_data.insert("hour".to_string(), StreamingQueryValue::String(hour_key));
            aggregated_row_data.insert(
                "record_count".to_string(),
                StreamingQueryValue::Integer(count),
            );
            aggregated_row_data.insert(
                "aggregated_at".to_string(),
                StreamingQueryValue::Timestamp(Utc::now()),
            );

            let mut aggregated_metadata = HashMap::new();
            aggregated_metadata.insert("aggregation_type".to_string(), "hourly_count".to_string());
            aggregated_metadata.insert("source_records".to_string(), count.to_string());

            let aggregated_row = StreamingQueryRow {
                id: Uuid::new_v4(),
                data: aggregated_row_data,
                metadata: aggregated_metadata,
                timestamp: first_row.timestamp,
            };

            aggregated_data.push(aggregated_row);
        }

        Ok(aggregated_data)
    }

    /// Update processing statistics
    async fn update_stats(&self, processing_time_ms: u64, record_count: usize, error_count: u64) {
        let mut stats = self.stats.write().await;

        stats.total_records_processed += record_count as u64;
        stats.total_processing_time_ms += processing_time_ms;
        stats.error_count += error_count;
        stats.last_processing_time = Some(Utc::now());

        // Calculate average processing time
        if stats.total_records_processed > 0 {
            stats.avg_processing_time_ms =
                stats.total_processing_time_ms as f64 / stats.total_records_processed as f64;
        }

        // Calculate records per second (simplified)
        if let Some(start_time) = self.start_time {
            let elapsed_seconds = (Utc::now() - start_time).num_seconds() as f64;
            if elapsed_seconds > 0.0 {
                stats.records_per_second = stats.total_records_processed as f64 / elapsed_seconds;
            }
        }
    }
}

#[async_trait]
impl StreamProcessor for DefaultStreamProcessor {
    async fn process(&self, data: Vec<StreamingQueryRow>) -> BridgeResult<StreamProcessingResult> {
        let start_time = std::time::Instant::now();
        let mut error_count = 0;

        info!("Processing {} records", data.len());

        // Apply processing pipeline
        let input_len = data.len();
        let mut processed_data = data;

        // Step 1: Apply filtering
        match self.apply_filtering(&processed_data).await {
            Ok(filtered_data) => {
                processed_data = filtered_data;
                info!(
                    "Applied filtering: {} records remaining",
                    processed_data.len()
                );
            }
            Err(e) => {
                error_count += 1;
                error!("Filtering failed: {}", e);
            }
        }

        // Step 2: Apply windowing
        match self.process_with_windowing(&processed_data).await {
            Ok(windowed_data) => {
                processed_data = windowed_data;
                info!(
                    "Applied windowing: {} records processed",
                    processed_data.len()
                );
            }
            Err(e) => {
                error_count += 1;
                error!("Windowing failed: {}", e);
            }
        }

        // Step 3: Apply transformations
        match self.apply_transformations(&processed_data).await {
            Ok(transformed_data) => {
                processed_data = transformed_data;
                info!(
                    "Applied transformations: {} records processed",
                    processed_data.len()
                );
            }
            Err(e) => {
                error_count += 1;
                error!("Transformation failed: {}", e);
            }
        }

        // Step 4: Apply aggregation
        match self.apply_aggregation(&processed_data).await {
            Ok(aggregated_data) => {
                processed_data = aggregated_data;
                info!(
                    "Applied aggregation: {} records processed",
                    processed_data.len()
                );
            }
            Err(e) => {
                error_count += 1;
                error!("Aggregation failed: {}", e);
            }
        }

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        // Update statistics
        self.update_stats(processing_time_ms, processed_data.len(), error_count)
            .await;

        // Create processing result
        let output_len = processed_data.len();
        let result = StreamProcessingResult {
            timestamp: Utc::now(),
            data: processed_data,
            stats: self.stats.read().await.clone(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert(
                    "processing_time_ms".to_string(),
                    processing_time_ms.to_string(),
                );
                metadata.insert("input_records".to_string(), input_len.to_string());
                metadata.insert("output_records".to_string(), output_len.to_string());
                metadata.insert("error_count".to_string(), error_count.to_string());
                metadata
            },
        };

        info!("Stream processing completed in {}ms", processing_time_ms);
        Ok(result)
    }

    async fn get_stats(&self) -> BridgeResult<StreamProcessingStats> {
        Ok(self.stats.read().await.clone())
    }

    async fn start(&mut self) -> BridgeResult<()> {
        if *self.is_running.read().await {
            warn!("Stream processor is already running");
            return Ok(());
        }

        info!("Starting stream processor: {}", self.config.name);

        // Initialize channels
        self.initialize_channels();

        // Set running state
        {
            let mut running = self.is_running.write().await;
            *running = true;
        }

        self.start_time = Some(Utc::now());

        // Start processing task
        let config = self.config.clone();
        let is_running = self.is_running.clone();
        let stats = self.stats.clone();
        let mut data_receiver = self.data_receiver.take();
        let result_sender = self.result_sender.clone();

        let task_handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(config.processing_interval_ms));

            while *is_running.read().await {
                interval.tick().await;

                // Process any available data
                if let Some(receiver) = &mut data_receiver {
                    while let Ok(data) = receiver.try_recv() {
                        let processor = DefaultStreamProcessor::new(config.clone());
                        match processor.process(data).await {
                            Ok(result) => {
                                if let Some(sender) = &result_sender {
                                    if let Err(e) = sender.send(result).await {
                                        error!("Failed to send processing result: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to process data: {}", e);
                                let mut stats_guard = stats.write().await;
                                stats_guard.error_count += 1;
                            }
                        }
                    }
                }
            }
        });

        self.task_handle = Some(task_handle);

        info!("Stream processor started successfully");
        Ok(())
    }

    async fn stop(&mut self) -> BridgeResult<()> {
        if !*self.is_running.read().await {
            warn!("Stream processor is not running");
            return Ok(());
        }

        info!("Stopping stream processor: {}", self.config.name);

        // Set running state to false
        {
            let mut running = self.is_running.write().await;
            *running = false;
        }

        // Wait for task to complete
        if let Some(handle) = self.task_handle.take() {
            if let Err(e) = handle.await {
                error!("Error waiting for processing task: {}", e);
            }
        }

        // Clear channels
        self.data_sender = None;
        self.data_receiver = None;
        self.result_sender = None;

        info!("Stream processor stopped successfully");
        Ok(())
    }

    fn is_running(&self) -> bool {
        *self.is_running.blocking_read()
    }
}
