//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Filter processor for the OpenTelemetry Data Lake Bridge
//!
//! This module provides a filter processor that can filter telemetry records
//! based on various criteria such as attributes, tags, and data content.

use crate::error::BridgeResult;
use crate::health::checker::HealthCheckCallback;
use crate::health::types::HealthCheckResult;
use crate::traits::processor::TelemetryProcessor;
use crate::types::{
    Filter, FilterOperator, FilterValue, ProcessedBatch, ProcessedRecord, ProcessingStatus,
    TelemetryBatch, TelemetryRecord,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Filter processor configuration
#[derive(Debug, Clone)]
pub struct FilterProcessorConfig {
    /// List of filters to apply
    pub filters: Vec<Filter>,

    /// Whether to include records that match any filter (OR) or all filters (AND)
    pub match_any: bool,

    /// Whether to drop records that don't match (true) or keep them (false)
    pub drop_unmatched: bool,

    /// Maximum number of records to process per batch
    pub max_records_per_batch: Option<usize>,
}

impl Default for FilterProcessorConfig {
    fn default() -> Self {
        Self {
            filters: Vec::new(),
            match_any: true,
            drop_unmatched: false,
            max_records_per_batch: None,
        }
    }
}

/// Filter processor statistics
#[derive(Debug, Clone)]
pub struct FilterProcessorStats {
    /// Total records processed
    pub total_records_processed: u64,

    /// Total records filtered out
    pub total_records_filtered: u64,

    /// Total records passed through
    pub total_records_passed: u64,

    /// Processing errors
    pub error_count: u64,

    /// Total batches processed
    pub total_batches: u64,

    /// Records per minute (rolling average)
    pub records_per_minute: u64,

    /// Batches per minute (rolling average)
    pub batches_per_minute: u64,

    /// Average processing time per record in milliseconds
    pub avg_processing_time_ms: f64,

    /// Last processing timestamp
    pub last_process_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Processing time tracking for rolling averages
    pub processing_times: Vec<std::time::Duration>,

    /// Timestamp tracking for rate calculations
    pub processing_timestamps: Vec<chrono::DateTime<chrono::Utc>>,
}

impl Default for FilterProcessorStats {
    fn default() -> Self {
        Self {
            total_records_processed: 0,
            total_records_filtered: 0,
            total_records_passed: 0,
            error_count: 0,
            total_batches: 0,
            records_per_minute: 0,
            batches_per_minute: 0,
            avg_processing_time_ms: 0.0,
            last_process_time: None,
            processing_times: Vec::new(),
            processing_timestamps: Vec::new(),
        }
    }
}

/// Filter processor for filtering telemetry records
pub struct FilterProcessor {
    /// Processor configuration
    config: FilterProcessorConfig,

    /// Processor statistics
    stats: Arc<RwLock<FilterProcessorStats>>,

    /// Running state
    running: Arc<RwLock<bool>>,
}

impl FilterProcessor {
    /// Create a new filter processor
    pub fn new(config: FilterProcessorConfig) -> Self {
        info!(
            "Creating filter processor with {} filters",
            config.filters.len()
        );

        Self {
            config,
            stats: Arc::new(RwLock::new(FilterProcessorStats::default())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the filter processor
    pub async fn start(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(crate::error::BridgeError::internal(
                "Filter processor is already running",
            ));
        }

        *running = true;
        info!("Starting filter processor");

        Ok(())
    }

    /// Stop the filter processor
    pub async fn stop(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Filter processor is not running",
            ));
        }

        *running = false;
        info!("Stopping filter processor");

        Ok(())
    }

    /// Apply filters to a single record
    fn apply_filters(&self, record: &TelemetryRecord) -> bool {
        if self.config.filters.is_empty() {
            return !self.config.drop_unmatched;
        }

        let mut matches = Vec::new();

        for filter in &self.config.filters {
            let matches_filter = self.evaluate_filter(record, filter);
            matches.push(matches_filter);
        }

        let result = if self.config.match_any {
            matches.iter().any(|&m| m)
        } else {
            matches.iter().all(|&m| m)
        };

        if self.config.drop_unmatched {
            result
        } else {
            !result
        }
    }

    /// Evaluate a single filter against a record
    fn evaluate_filter(&self, record: &TelemetryRecord, filter: &Filter) -> bool {
        let value = match filter.field.as_str() {
            "record_type" => Some(FilterValue::String(format!("{:?}", record.record_type))),
            "source" => record
                .attributes
                .get(&filter.field)
                .map(|v| FilterValue::String(v.clone())),
            "service" => {
                // Check service field first, then attributes
                if let Some(service) = record.service.as_ref() {
                    Some(FilterValue::String(service.name.clone()))
                } else if let Some(attr_value) = record.attributes.get(&filter.field) {
                    Some(FilterValue::String(attr_value.clone()))
                } else {
                    None
                }
            }
            "resource" => record
                .resource
                .as_ref()
                .map(|r| FilterValue::String(r.resource_type.clone())),
            _ => {
                // Check attributes first, then tags
                if let Some(attr_value) = record.attributes.get(&filter.field) {
                    Some(FilterValue::String(attr_value.clone()))
                } else if let Some(tag_value) = record.tags.get(&filter.field) {
                    Some(FilterValue::String(tag_value.clone()))
                } else {
                    None
                }
            }
        };

        if let Some(actual_value) = value {
            self.compare_values(&actual_value, &filter.operator, &filter.value)
        } else {
            false
        }
    }

    /// Compare two filter values
    fn compare_values(
        &self,
        actual: &FilterValue,
        operator: &FilterOperator,
        expected: &FilterValue,
    ) -> bool {
        match (actual, operator, expected) {
            (
                FilterValue::String(actual_str),
                FilterOperator::Equals,
                FilterValue::String(expected_str),
            ) => actual_str == expected_str,
            (
                FilterValue::String(actual_str),
                FilterOperator::NotEquals,
                FilterValue::String(expected_str),
            ) => actual_str != expected_str,
            (
                FilterValue::String(actual_str),
                FilterOperator::Contains,
                FilterValue::String(expected_str),
            ) => actual_str.contains(expected_str),
            (
                FilterValue::String(actual_str),
                FilterOperator::StartsWith,
                FilterValue::String(expected_str),
            ) => actual_str.starts_with(expected_str),
            (
                FilterValue::String(actual_str),
                FilterOperator::EndsWith,
                FilterValue::String(expected_str),
            ) => actual_str.ends_with(expected_str),
            (
                FilterValue::Number(actual_num),
                FilterOperator::Equals,
                FilterValue::Number(expected_num),
            ) => (actual_num - expected_num).abs() < f64::EPSILON,
            (
                FilterValue::Number(actual_num),
                FilterOperator::NotEquals,
                FilterValue::Number(expected_num),
            ) => (actual_num - expected_num).abs() >= f64::EPSILON,
            (
                FilterValue::Number(actual_num),
                FilterOperator::GreaterThan,
                FilterValue::Number(expected_num),
            ) => actual_num > expected_num,
            (
                FilterValue::Number(actual_num),
                FilterOperator::LessThan,
                FilterValue::Number(expected_num),
            ) => actual_num < expected_num,
            (
                FilterValue::Boolean(actual_bool),
                FilterOperator::Equals,
                FilterValue::Boolean(expected_bool),
            ) => actual_bool == expected_bool,
            _ => false, // Type mismatch or unsupported operation
        }
    }

    /// Update rolling averages for rates and processing times
    fn update_rolling_averages(
        &self,
        stats: &mut FilterProcessorStats,
        processing_time: std::time::Duration,
        record_count: usize,
    ) {
        let now = chrono::Utc::now();

        // Add current processing time and timestamp
        stats.processing_times.push(processing_time);
        stats.processing_timestamps.push(now);

        // Keep only last 60 entries for rolling average (1 minute window)
        const ROLLING_WINDOW_SIZE: usize = 60;
        if stats.processing_times.len() > ROLLING_WINDOW_SIZE {
            stats.processing_times.remove(0);
            stats.processing_timestamps.remove(0);
        }

        // Calculate records per minute
        if stats.processing_timestamps.len() >= 2 {
            let time_span = stats
                .processing_timestamps
                .last()
                .unwrap()
                .signed_duration_since(*stats.processing_timestamps.first().unwrap())
                .num_seconds() as f64;

            if time_span > 0.0 {
                let total_records_in_window = stats.processing_times.len() * record_count; // Approximate
                stats.records_per_minute =
                    ((total_records_in_window as f64 / time_span) * 60.0) as u64;
            }
        }

        // Calculate batches per minute
        if stats.processing_timestamps.len() >= 2 {
            let time_span = stats
                .processing_timestamps
                .last()
                .unwrap()
                .signed_duration_since(*stats.processing_timestamps.first().unwrap())
                .num_seconds() as f64;

            if time_span > 0.0 {
                stats.batches_per_minute =
                    ((stats.processing_timestamps.len() as f64 / time_span) * 60.0) as u64;
            }
        }

        // Calculate average processing time per record
        if !stats.processing_times.is_empty() {
            let total_processing_time: std::time::Duration = stats.processing_times.iter().sum();
            let total_records_processed = stats.total_records_processed;

            if total_records_processed > 0 {
                stats.avg_processing_time_ms =
                    total_processing_time.as_millis() as f64 / total_records_processed as f64;
            }
        }
    }
}

#[async_trait]
impl TelemetryProcessor for FilterProcessor {
    async fn process(&self, batch: TelemetryBatch) -> BridgeResult<ProcessedBatch> {
        let running = self.running.read().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Filter processor is not running",
            ));
        }

        let start_time = std::time::Instant::now();
        let mut processed_records = Vec::new();
        let mut filtered_count = 0;
        let mut passed_count = 0;

        let record_count = batch.records.len();
        for record in &batch.records {
            let should_keep = self.apply_filters(&record);

            if should_keep {
                processed_records.push(ProcessedRecord {
                    original_id: record.id,
                    status: ProcessingStatus::Success,
                    transformed_data: Some(record.data.clone()),
                    metadata: record.attributes.clone(),
                    errors: vec![],
                });
                passed_count += 1;
            } else {
                filtered_count += 1;
            }
        }

        // Apply max records limit if configured
        if let Some(max_records) = self.config.max_records_per_batch {
            if processed_records.len() > max_records {
                processed_records.truncate(max_records);
                debug!("Truncated batch to {} records", max_records);
            }
        }

        let processing_time = start_time.elapsed();

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_records_processed += record_count as u64;
            stats.total_records_filtered += filtered_count as u64;
            stats.total_records_passed += passed_count as u64;
            stats.total_batches += 1;
            stats.last_process_time = Some(chrono::Utc::now());

            // Update rolling averages
            self.update_rolling_averages(&mut stats, processing_time, record_count);
        }
        debug!(
            "Filtered batch: {} records processed, {} passed, {} filtered in {:?}",
            batch.records.len(),
            passed_count,
            filtered_count,
            processing_time
        );

        Ok(ProcessedBatch {
            original_batch_id: batch.id,
            timestamp: batch.timestamp,
            status: ProcessingStatus::Success,
            records: processed_records,
            metadata: batch.metadata,
            errors: vec![],
        })
    }

    fn name(&self) -> &str {
        "filter_processor"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        let running = self.running.read().await;
        Ok(*running)
    }

    async fn get_stats(&self) -> BridgeResult<crate::traits::processor::ProcessorStats> {
        let stats = self.stats.read().await;

        Ok(crate::traits::processor::ProcessorStats {
            total_batches: stats.total_batches,
            total_records: stats.total_records_processed,
            batches_per_minute: stats.batches_per_minute,
            records_per_minute: stats.records_per_minute,
            avg_processing_time_ms: stats.avg_processing_time_ms,
            error_count: stats.error_count,
            last_process_time: stats.last_process_time,
        })
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        self.stop().await
    }
}

#[async_trait]
impl HealthCheckCallback for FilterProcessor {
    async fn check(&self) -> BridgeResult<HealthCheckResult> {
        let running = self.running.read().await;

        Ok(HealthCheckResult {
            component: "filter_processor".to_string(),
            status: if *running {
                crate::health::types::HealthStatus::Healthy
            } else {
                crate::health::types::HealthStatus::Unhealthy
            },
            timestamp: chrono::Utc::now(),
            duration_ms: 0,
            message: if *running {
                "Filter processor is running".to_string()
            } else {
                "Filter processor is not running".to_string()
            },
            details: None,
            errors: vec![],
        })
    }

    fn component_name(&self) -> &str {
        "filter_processor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MetricData, MetricType, MetricValue, TelemetryData, TelemetryType};

    #[tokio::test]
    async fn test_filter_processor_creation() {
        let config = FilterProcessorConfig::default();
        let processor = FilterProcessor::new(config);

        let stats = processor.get_stats().await.unwrap();
        assert_eq!(stats.total_records, 0);
    }

    #[tokio::test]
    async fn test_filter_processor_start_stop() {
        let config = FilterProcessorConfig::default();
        let processor = FilterProcessor::new(config);

        // Start the processor
        let result = processor.start().await;
        assert!(result.is_ok());

        // Check health
        let health = processor.health_check().await.unwrap();
        assert!(health);

        // Stop the processor
        let result = processor.stop().await;
        assert!(result.is_ok());

        // Check health again
        let health = processor.health_check().await.unwrap();
        assert!(!health);
    }

    #[tokio::test]
    async fn test_filter_processor_no_filters() {
        let config = FilterProcessorConfig::default();
        let processor = FilterProcessor::new(config);
        processor.start().await.unwrap();

        let batch = create_test_batch();
        let result = processor.process(batch).await.unwrap();

        // Should pass all records when no filters are configured
        assert_eq!(result.records.len(), 2);
        assert_eq!(result.status, ProcessingStatus::Success);

        processor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_filter_processor_with_filters() {
        let config = FilterProcessorConfig {
            filters: vec![Filter {
                field: "service".to_string(),
                operator: FilterOperator::Equals,
                value: FilterValue::String("test_service".to_string()),
            }],
            match_any: true,
            drop_unmatched: true,
            max_records_per_batch: None,
        };

        let processor = FilterProcessor::new(config);
        processor.start().await.unwrap();

        let batch = create_test_batch();
        let result = processor.process(batch).await.unwrap();

        // Should filter out records that don't match
        assert_eq!(result.records.len(), 1);
        assert_eq!(result.status, ProcessingStatus::Success);

        processor.stop().await.unwrap();
    }

    fn create_test_batch() -> TelemetryBatch {
        let record1 = TelemetryRecord::new(
            TelemetryType::Metric,
            TelemetryData::Metric(MetricData {
                name: "test_metric".to_string(),
                description: None,
                unit: None,
                metric_type: MetricType::Counter,
                value: MetricValue::Counter(1.0),
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            }),
        )
        .with_attribute("service".to_string(), "test_service".to_string());

        let record2 = TelemetryRecord::new(
            TelemetryType::Metric,
            TelemetryData::Metric(MetricData {
                name: "test_metric2".to_string(),
                description: None,
                unit: None,
                metric_type: MetricType::Counter,
                value: MetricValue::Counter(2.0),
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            }),
        )
        .with_attribute("service".to_string(), "other_service".to_string());

        TelemetryBatch::new("test_source".to_string(), vec![record1, record2])
    }

    #[tokio::test]
    async fn test_statistics_tracking() {
        let config = FilterProcessorConfig::default();
        let processor = FilterProcessor::new(config);

        // Check initial stats
        let stats = processor.get_stats().await.unwrap();
        assert_eq!(stats.total_batches, 0);
        assert_eq!(stats.total_records, 0);
        assert_eq!(stats.batches_per_minute, 0);
        assert_eq!(stats.records_per_minute, 0);
        assert_eq!(stats.avg_processing_time_ms, 0.0);

        processor.start().await.unwrap();

        // Process a batch
        let batch = create_test_batch();
        let result = processor.process(batch).await.unwrap();

        // Check updated stats
        let stats = processor.get_stats().await.unwrap();
        assert_eq!(stats.total_batches, 1);
        assert_eq!(stats.total_records, 2); // 2 records in test batch
        assert!(stats.avg_processing_time_ms >= 0.0);

        // Process another batch
        let batch2 = create_test_batch();
        let result = processor.process(batch2).await.unwrap();

        // Check final stats
        let stats = processor.get_stats().await.unwrap();
        assert_eq!(stats.total_batches, 2);
        assert_eq!(stats.total_records, 4); // 4 total records
        assert!(stats.avg_processing_time_ms >= 0.0);

        processor.stop().await.unwrap();
    }
}
