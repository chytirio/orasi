//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Data processing utilities for the OpenTelemetry Data Lake Bridge

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::BridgeResult;

/// Data processing configuration
#[derive(Debug, Clone)]
pub struct ProcessingConfig {
    /// Enable data validation
    pub enable_validation: bool,

    /// Enable data transformation
    pub enable_transformation: bool,

    /// Enable data filtering
    pub enable_filtering: bool,

    /// Batch size for processing
    pub batch_size: usize,

    /// Processing timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            enable_validation: true,
            enable_transformation: true,
            enable_filtering: true,
            batch_size: 1000,
            timeout_ms: 5000,
        }
    }
}

/// Data processing statistics
#[derive(Debug, Clone)]
pub struct ProcessingStats {
    /// Total records processed
    pub total_records: u64,

    /// Valid records
    pub valid_records: u64,

    /// Invalid records
    pub invalid_records: u64,

    /// Transformed records
    pub transformed_records: u64,

    /// Filtered records
    pub filtered_records: u64,

    /// Processing errors
    pub processing_errors: u64,

    /// Average processing time per record in milliseconds
    pub avg_processing_time_ms: f64,
}

impl Default for ProcessingStats {
    fn default() -> Self {
        Self {
            total_records: 0,
            valid_records: 0,
            invalid_records: 0,
            transformed_records: 0,
            filtered_records: 0,
            processing_errors: 0,
            avg_processing_time_ms: 0.0,
        }
    }
}

/// Data processor
pub struct DataProcessor {
    config: ProcessingConfig,
    stats: Arc<RwLock<ProcessingStats>>,
}

impl DataProcessor {
    /// Create a new data processor
    pub fn new(config: ProcessingConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(ProcessingStats::default())),
        }
    }

    /// Process a batch of data
    pub async fn process_batch<T>(&self, data: Vec<T>) -> BridgeResult<Vec<T>>
    where
        T: Clone + Send + Sync + 'static,
    {
        let start_time = std::time::Instant::now();
        let mut processed_data = data;

        // Validate data if enabled
        if self.config.enable_validation {
            processed_data = self.validate_data(processed_data).await?;
        }

        // Transform data if enabled
        if self.config.enable_transformation {
            processed_data = self.transform_data(processed_data).await?;
        }

        // Filter data if enabled
        if self.config.enable_filtering {
            processed_data = self.filter_data(processed_data).await?;
        }

        // Update statistics
        self.update_stats(processed_data.len(), start_time.elapsed())
            .await;

        Ok(processed_data)
    }

    /// Validate data
    async fn validate_data<T>(&self, data: Vec<T>) -> BridgeResult<Vec<T>>
    where
        T: Clone + Send + Sync + 'static,
    {
        // Placeholder validation logic
        // In a real implementation, this would validate the data structure
        Ok(data)
    }

    /// Transform data
    async fn transform_data<T>(&self, data: Vec<T>) -> BridgeResult<Vec<T>>
    where
        T: Clone + Send + Sync + 'static,
    {
        // Placeholder transformation logic
        // In a real implementation, this would transform the data
        Ok(data)
    }

    /// Filter data
    async fn filter_data<T>(&self, data: Vec<T>) -> BridgeResult<Vec<T>>
    where
        T: Clone + Send + Sync + 'static,
    {
        // Placeholder filtering logic
        // In a real implementation, this would filter the data
        Ok(data)
    }

    /// Update processing statistics
    async fn update_stats(&self, record_count: usize, processing_time: std::time::Duration) {
        let mut stats = self.stats.write().await;
        stats.total_records += record_count as u64;
        stats.valid_records += record_count as u64;
        stats.transformed_records += record_count as u64;
        stats.filtered_records += record_count as u64;

        let processing_time_ms = processing_time.as_millis() as f64;
        if stats.total_records > 0 {
            stats.avg_processing_time_ms = (stats.avg_processing_time_ms
                * (stats.total_records - record_count as u64) as f64
                + processing_time_ms)
                / stats.total_records as f64;
        }
    }

    /// Get processing statistics
    pub async fn get_stats(&self) -> ProcessingStats {
        self.stats.read().await.clone()
    }

    /// Reset processing statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = ProcessingStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_data_processor_creation() {
        let config = ProcessingConfig::default();
        let processor = DataProcessor::new(config);
        let stats = processor.get_stats().await;
        assert_eq!(stats.total_records, 0);
    }

    #[tokio::test]
    async fn test_process_batch() {
        let config = ProcessingConfig::default();
        let processor = DataProcessor::new(config);

        let data = vec![1, 2, 3, 4, 5];
        let result = processor.process_batch(data).await;
        assert!(result.is_ok());

        let stats = processor.get_stats().await;
        assert_eq!(stats.total_records, 5);
    }
}
