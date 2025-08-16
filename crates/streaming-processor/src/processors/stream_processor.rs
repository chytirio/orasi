//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Stream processor implementation
//!
//! This module provides a basic stream processor for streaming data.

use async_trait::async_trait;
use bridge_core::{
    traits::{DataStream, StreamProcessor as BridgeStreamProcessor, StreamProcessorStats},
    BridgeResult,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use super::ProcessorConfig;

/// Stream processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamProcessorConfig {
    /// Processor name
    pub name: String,

    /// Processor version
    pub version: String,

    /// Batch size for processing
    pub batch_size: usize,

    /// Buffer size for incoming data
    pub buffer_size: usize,

    /// Processing timeout in milliseconds
    pub processing_timeout_ms: u64,

    /// Enable parallel processing
    pub enable_parallel_processing: bool,

    /// Number of parallel workers
    pub num_workers: usize,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl StreamProcessorConfig {
    /// Create new stream processor configuration
    pub fn new() -> Self {
        Self {
            name: "stream".to_string(),
            version: "1.0.0".to_string(),
            batch_size: 1000,
            buffer_size: 10000,
            processing_timeout_ms: 5000,
            enable_parallel_processing: false,
            num_workers: 1,
            additional_config: HashMap::new(),
        }
    }
}

#[async_trait]
impl ProcessorConfig for StreamProcessorConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.batch_size == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Batch size cannot be 0".to_string(),
            ));
        }

        if self.num_workers == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Number of workers cannot be 0".to_string(),
            ));
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Stream processor implementation
pub struct StreamProcessor {
    config: StreamProcessorConfig,
    stats: Arc<RwLock<StreamProcessorStats>>,
}

impl StreamProcessor {
    /// Create new stream processor
    pub async fn new(config: &dyn ProcessorConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<StreamProcessorConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration(
                    "Invalid stream processor configuration".to_string(),
                )
            })?
            .clone();

        config.validate().await?;

        let stats = StreamProcessorStats {
            total_records: 0,
            records_per_minute: 0,
            avg_processing_time_ms: 0.0,
            error_count: 0,
            last_process_time: None,
        };

        Ok(Self {
            config,
            stats: Arc::new(RwLock::new(stats)),
        })
    }

    /// Process data stream with comprehensive streaming logic
    async fn process_data_stream(&self, input: DataStream) -> BridgeResult<DataStream> {
        let start_time = std::time::Instant::now();

        info!("Processing data stream: {}", input.stream_id);

        // Step 1: Data validation
        let validated_data = self.validate_data_stream(&input).await?;

        // Step 2: Basic transformations
        let transformed_data = self.apply_transformations(&validated_data).await?;

        // Step 3: Quality checks
        let quality_checked_data = self.perform_quality_checks(&transformed_data).await?;

        // Step 4: Metadata enrichment
        let enriched_metadata = self
            .enrich_metadata(&input.metadata, &quality_checked_data)
            .await?;

        let processed_stream = DataStream {
            stream_id: input.stream_id,
            data: quality_checked_data,
            metadata: enriched_metadata,
            timestamp: Utc::now(),
        };

        let processing_time = start_time.elapsed().as_millis() as f64;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_records += 1; // Simplified - in practice would count actual records
            stats.avg_processing_time_ms = (stats.avg_processing_time_ms + processing_time) / 2.0;
            stats.last_process_time = Some(Utc::now());
        }

        info!("Data stream processed in {}ms", processing_time);

        Ok(processed_stream)
    }

    /// Validate incoming data stream
    async fn validate_data_stream(&self, input: &DataStream) -> BridgeResult<Vec<u8>> {
        // Basic validation: check if data is not empty and has reasonable size
        if input.data.is_empty() {
            return Err(bridge_core::BridgeError::validation(
                "Data stream is empty".to_string(),
            ));
        }

        if input.data.len() > self.config.buffer_size {
            warn!(
                "Data stream size ({}) exceeds buffer size ({})",
                input.data.len(),
                self.config.buffer_size
            );
        }

        // Check for basic data integrity (simple checksum or magic number validation)
        if input.data.len() >= 4 {
            // Simple validation: check if data starts with common formats
            let is_valid_format = input.data.starts_with(b"{") || // JSON
                                 input.data.starts_with(b"<") || // XML
                                 input.data.starts_with(b"PAR1") || // Parquet
                                 input.data.iter().any(|&b| b.is_ascii_alphanumeric()); // Text

            if !is_valid_format {
                warn!("Data stream format may be invalid or corrupted");
            }
        }

        Ok(input.data.clone())
    }

    /// Apply basic transformations to the data
    async fn apply_transformations(&self, data: &[u8]) -> BridgeResult<Vec<u8>> {
        let mut transformed_data = data.to_vec();

        // Apply transformations based on configuration
        if self.config.enable_parallel_processing {
            // For parallel processing, we might split the data into chunks
            // For now, just add a transformation marker
            transformed_data.extend_from_slice(b"_transformed");
        }

        // Basic data normalization (remove null bytes, trim whitespace if text)
        if transformed_data.iter().any(|&b| b == 0) {
            transformed_data.retain(|&b| b != 0);
            info!("Removed null bytes from data stream");
        }

        // Add processing timestamp to data if it's JSON
        if transformed_data.starts_with(b"{") {
            if let Ok(json_str) = String::from_utf8(transformed_data.clone()) {
                if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(obj) = json.as_object_mut() {
                        obj.insert(
                            "_processed_at".to_string(),
                            serde_json::Value::String(Utc::now().to_rfc3339()),
                        );
                        if let Ok(new_json) = serde_json::to_vec(&json) {
                            transformed_data = new_json;
                        }
                    }
                }
            }
        }

        Ok(transformed_data)
    }

    /// Perform quality checks on the data
    async fn perform_quality_checks(&self, data: &[u8]) -> BridgeResult<Vec<u8>> {
        let mut quality_score = 100.0;
        let mut issues = Vec::new();

        // Check data size
        if data.len() < 10 {
            quality_score -= 20.0;
            issues.push("Data size is very small".to_string());
        }

        // Check for encoding issues
        if let Err(_) = String::from_utf8(data.to_vec()) {
            quality_score -= 30.0;
            issues.push("Data contains invalid UTF-8 sequences".to_string());
        }

        // Check for repeated patterns (potential data corruption)
        if data.len() > 100 {
            let chunk_size = data.len() / 4;
            let chunk1 = &data[0..chunk_size];
            let chunk2 = &data[chunk_size..chunk_size * 2];
            if chunk1 == chunk2 {
                quality_score -= 25.0;
                issues.push("Data contains repeated patterns".to_string());
            }
        }

        // Log quality issues
        if !issues.is_empty() {
            warn!("Data quality issues detected: {:?}", issues);
        }

        if quality_score < 50.0 {
            return Err(bridge_core::BridgeError::validation(format!(
                "Data quality too low ({}): {:?}",
                quality_score, issues
            )));
        }

        info!("Data quality check passed with score: {}", quality_score);
        Ok(data.to_vec())
    }

    /// Enrich metadata with processing information
    async fn enrich_metadata(
        &self,
        original_metadata: &HashMap<String, String>,
        processed_data: &[u8],
    ) -> BridgeResult<HashMap<String, String>> {
        let mut enriched_metadata = original_metadata.clone();

        // Add processing information
        enriched_metadata.insert("processed_by".to_string(), self.config.name.clone());
        enriched_metadata.insert("processed_at".to_string(), Utc::now().to_rfc3339());
        enriched_metadata.insert("processor_version".to_string(), self.config.version.clone());
        enriched_metadata.insert(
            "processing_batch_size".to_string(),
            self.config.batch_size.to_string(),
        );

        // Add data statistics
        enriched_metadata.insert(
            "data_size_bytes".to_string(),
            processed_data.len().to_string(),
        );
        enriched_metadata.insert(
            "data_format".to_string(),
            self.detect_data_format(processed_data),
        );

        // Add processing configuration
        enriched_metadata.insert(
            "parallel_processing".to_string(),
            self.config.enable_parallel_processing.to_string(),
        );
        enriched_metadata.insert(
            "num_workers".to_string(),
            self.config.num_workers.to_string(),
        );

        // Add quality metrics
        let quality_score = self.calculate_quality_score(processed_data);
        enriched_metadata.insert("quality_score".to_string(), format!("{:.2}", quality_score));

        Ok(enriched_metadata)
    }

    /// Detect the format of the data
    fn detect_data_format(&self, data: &[u8]) -> String {
        if data.is_empty() {
            return "unknown".to_string();
        }

        if data.starts_with(b"{") || data.starts_with(b"[") {
            "json".to_string()
        } else if data.starts_with(b"<") {
            "xml".to_string()
        } else if data.starts_with(b"PAR1") {
            "parquet".to_string()
        } else if data.starts_with(b"\x1f\x8b") {
            "gzip".to_string()
        } else if data
            .iter()
            .all(|&b| b.is_ascii() || b.is_ascii_whitespace())
        {
            "text".to_string()
        } else {
            "binary".to_string()
        }
    }

    /// Calculate a simple quality score for the data
    fn calculate_quality_score(&self, data: &[u8]) -> f64 {
        let mut score = 100.0;

        // Penalize empty data
        if data.is_empty() {
            score -= 100.0;
        }

        // Penalize very small data
        if data.len() < 10 {
            score -= 20.0;
        }

        // Bonus for valid JSON
        if data.starts_with(b"{") || data.starts_with(b"[") {
            if let Ok(_) = serde_json::from_slice::<serde_json::Value>(data) {
                score += 10.0;
            }
        }

        // Penalize for null bytes
        let null_count = data.iter().filter(|&&b| b == 0).count();
        if null_count > 0 {
            score -= (null_count as f64 / data.len() as f64) * 50.0;
        }

        score.max(0.0).min(100.0)
    }
}

#[async_trait]
impl BridgeStreamProcessor for StreamProcessor {
    async fn process_stream(&self, input: DataStream) -> BridgeResult<DataStream> {
        self.process_data_stream(input).await
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn version(&self) -> &str {
        &self.config.version
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        // Simple health check - processor is healthy if it has a valid config
        Ok(true)
    }

    async fn get_stats(&self) -> BridgeResult<StreamProcessorStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down stream processor: {}", self.config.name);
        Ok(())
    }
}

impl StreamProcessor {
    /// Get stream processor configuration
    pub fn get_config(&self) -> &StreamProcessorConfig {
        &self.config
    }
}
