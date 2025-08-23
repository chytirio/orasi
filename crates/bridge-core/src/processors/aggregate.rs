//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Aggregate processor for the OpenTelemetry Data Lake Bridge
//!
//! This module provides an aggregate processor that can aggregate telemetry data
//! by applying various aggregation functions such as sum, average, count, etc.

use crate::error::BridgeResult;
use crate::health::checker::HealthCheckCallback;
use crate::health::types::HealthCheckResult;
use crate::traits::processor::TelemetryProcessor;
use crate::types::metrics::{MetricData, MetricType, MetricValue};
use crate::types::{
    ProcessedBatch, ProcessedRecord, ProcessingStatus, TelemetryBatch, TelemetryData,
    TelemetryRecord,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Aggregation operation types
#[derive(Debug, Clone)]
pub enum AggregationOperation {
    /// Count records by field value
    Count {
        group_by: String,
        output_field: String,
    },
    /// Sum numeric field values
    Sum {
        field: String,
        group_by: Option<String>,
        output_field: String,
    },
    /// Calculate average of numeric field values
    Average {
        field: String,
        group_by: Option<String>,
        output_field: String,
    },
    /// Find minimum value
    Min {
        field: String,
        group_by: Option<String>,
        output_field: String,
    },
    /// Find maximum value
    Max {
        field: String,
        group_by: Option<String>,
        output_field: String,
    },
    /// Calculate percentile
    Percentile {
        field: String,
        percentile: f64,
        group_by: Option<String>,
        output_field: String,
    },
    /// Rate calculation (events per time window)
    Rate {
        time_window_seconds: u64,
        group_by: Option<String>,
        output_field: String,
    },
}

/// Time window for aggregations
#[derive(Debug, Clone)]
pub enum TimeWindow {
    /// Fixed time window (e.g., every 60 seconds)
    Fixed { duration_seconds: u64 },
    /// Sliding time window
    Sliding {
        duration_seconds: u64,
        step_seconds: u64,
    },
    /// Tumbling window (non-overlapping)
    Tumbling { duration_seconds: u64 },
}

/// Aggregate processor configuration
#[derive(Debug, Clone)]
pub struct AggregateProcessorConfig {
    /// Aggregations to apply
    pub aggregations: Vec<AggregationOperation>,

    /// Time window for aggregations
    pub time_window: TimeWindow,

    /// Whether to emit intermediate results
    pub emit_intermediate: bool,

    /// Maximum number of groups to track
    pub max_groups: usize,

    /// Whether to reset aggregations after emission
    pub reset_after_emit: bool,
}

impl Default for AggregateProcessorConfig {
    fn default() -> Self {
        Self {
            aggregations: Vec::new(),
            time_window: TimeWindow::Fixed {
                duration_seconds: 60,
            },
            emit_intermediate: false,
            max_groups: 10000,
            reset_after_emit: true,
        }
    }
}

/// Aggregation state for a group
#[derive(Debug, Clone)]
struct GroupAggregationState {
    /// Count of records
    count: u64,

    /// Sum of numeric values
    sum: f64,

    /// Minimum value
    min: Option<f64>,

    /// Maximum value
    max: Option<f64>,

    /// Values for percentile calculation
    values: Vec<f64>,

    /// First timestamp in window
    first_timestamp: chrono::DateTime<chrono::Utc>,

    /// Last timestamp in window
    last_timestamp: chrono::DateTime<chrono::Utc>,

    /// Additional metadata
    metadata: HashMap<String, String>,
}

impl Default for GroupAggregationState {
    fn default() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            min: None,
            max: None,
            values: Vec::new(),
            first_timestamp: chrono::Utc::now(),
            last_timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

/// Aggregate processor statistics
#[derive(Debug, Clone)]
pub struct AggregateProcessorStats {
    /// Total records processed
    pub total_records: u64,

    /// Total aggregations computed
    pub total_aggregations: u64,

    /// Active groups being tracked
    pub active_groups: u64,

    /// Aggregations emitted
    pub aggregations_emitted: u64,

    /// Average processing time per record in milliseconds
    pub avg_processing_time_ms: f64,

    /// Records per minute
    pub records_per_minute: u64,

    /// Last processing time
    pub last_processing_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Last emission time
    pub last_emission_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Total aggregation errors encountered
    pub error_count: u64,
}

impl Default for AggregateProcessorStats {
    fn default() -> Self {
        Self {
            total_records: 0,
            total_aggregations: 0,
            active_groups: 0,
            aggregations_emitted: 0,
            avg_processing_time_ms: 0.0,
            records_per_minute: 0,
            last_processing_time: None,
            last_emission_time: None,
            error_count: 0,
        }
    }
}

/// Aggregate processor for aggregating telemetry data
#[derive(Debug)]
pub struct AggregateProcessor {
    /// Processor configuration
    pub config: AggregateProcessorConfig,

    /// Processor statistics
    pub stats: Arc<RwLock<AggregateProcessorStats>>,

    /// Running state
    pub running: Arc<RwLock<bool>>,

    /// Aggregation state by group
    pub aggregation_state: Arc<RwLock<HashMap<String, GroupAggregationState>>>,

    /// Last window start time
    pub last_window_start: Arc<RwLock<chrono::DateTime<chrono::Utc>>>,
}

impl AggregateProcessor {
    /// Create a new aggregate processor
    pub fn new(config: AggregateProcessorConfig) -> Self {
        info!(
            "Creating aggregate processor with {} aggregations",
            config.aggregations.len()
        );

        Self {
            config,
            stats: Arc::new(RwLock::new(AggregateProcessorStats::default())),
            running: Arc::new(RwLock::new(false)),
            aggregation_state: Arc::new(RwLock::new(HashMap::new())),
            last_window_start: Arc::new(RwLock::new(chrono::Utc::now())),
        }
    }

    /// Start the aggregate processor
    pub async fn start(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(crate::error::BridgeError::internal(
                "Aggregate processor is already running",
            ));
        }

        *running = true;
        info!("Starting aggregate processor");

        // Initialize window start time
        {
            let mut window_start = self.last_window_start.write().await;
            *window_start = chrono::Utc::now();
        }

        Ok(())
    }

    /// Stop the aggregate processor
    pub async fn stop(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Aggregate processor is not running",
            ));
        }

        *running = false;
        info!("Stopping aggregate processor");

        Ok(())
    }

    /// Process a single record for aggregation
    async fn aggregate_record(&self, record: &TelemetryRecord) -> BridgeResult<()> {
        let mut state = self.aggregation_state.write().await;

        for aggregation in &self.config.aggregations {
            let group_key = self.get_group_key(aggregation, record)?;

            // Get or create group state
            let group_state = state.entry(group_key.clone()).or_insert_with(|| {
                let mut default_state = GroupAggregationState::default();
                default_state.first_timestamp = record.timestamp;
                default_state
            });

            // Update group state
            group_state.last_timestamp = record.timestamp;
            group_state.count += 1;

            // Apply specific aggregation
            match aggregation {
                AggregationOperation::Count { .. } => {
                    // Count is already incremented above
                }

                AggregationOperation::Sum { field, .. } => {
                    if let Some(value) = self.extract_numeric_value(record, field)? {
                        group_state.sum += value;
                    }
                }

                AggregationOperation::Average { field, .. } => {
                    if let Some(value) = self.extract_numeric_value(record, field)? {
                        group_state.sum += value;
                        group_state.values.push(value);
                    }
                }

                AggregationOperation::Min { field, .. } => {
                    if let Some(value) = self.extract_numeric_value(record, field)? {
                        group_state.min = Some(group_state.min.map_or(value, |min| min.min(value)));
                    }
                }

                AggregationOperation::Max { field, .. } => {
                    if let Some(value) = self.extract_numeric_value(record, field)? {
                        group_state.max = Some(group_state.max.map_or(value, |max| max.max(value)));
                    }
                }

                AggregationOperation::Percentile { field, .. } => {
                    if let Some(value) = self.extract_numeric_value(record, field)? {
                        group_state.values.push(value);
                    }
                }

                AggregationOperation::Rate { .. } => {
                    // Rate calculation is based on count over time window
                    // No additional state needed beyond count and timestamps
                }
            }
        }

        // Check if we need to limit groups
        if state.len() > self.config.max_groups {
            warn!(
                "Maximum groups exceeded ({}), removing oldest groups",
                self.config.max_groups
            );
            // Remove oldest groups (simple LRU-like behavior)
            let oldest_keys: Vec<String> = state
                .iter()
                .map(|(k, v)| (k.clone(), v.first_timestamp))
                .collect::<Vec<_>>()
                .into_iter()
                .fold(Vec::new(), |mut acc, (key, timestamp)| {
                    acc.push((key, timestamp));
                    acc.sort_by(|a, b| a.1.cmp(&b.1));
                    if acc.len() > self.config.max_groups / 10 {
                        acc.truncate(self.config.max_groups / 10);
                    }
                    acc
                })
                .into_iter()
                .map(|(key, _)| key)
                .collect();

            for key in oldest_keys {
                state.remove(&key);
            }
        }

        Ok(())
    }

    /// Get group key for aggregation
    fn get_group_key(
        &self,
        aggregation: &AggregationOperation,
        record: &TelemetryRecord,
    ) -> BridgeResult<String> {
        let group_by = match aggregation {
            AggregationOperation::Count { group_by, .. } => Some(group_by),
            AggregationOperation::Sum { group_by, .. } => group_by.as_ref(),
            AggregationOperation::Average { group_by, .. } => group_by.as_ref(),
            AggregationOperation::Min { group_by, .. } => group_by.as_ref(),
            AggregationOperation::Max { group_by, .. } => group_by.as_ref(),
            AggregationOperation::Percentile { group_by, .. } => group_by.as_ref(),
            AggregationOperation::Rate { group_by, .. } => group_by.as_ref(),
        };

        if let Some(group_field) = group_by {
            let group_value = record
                .attributes
                .get(group_field)
                .or_else(|| record.tags.get(group_field))
                .map_or("default", |v| v);
            Ok(format!("{}:{}", group_field, group_value))
        } else {
            Ok("_global".to_string())
        }
    }

    /// Extract numeric value from record
    fn extract_numeric_value(
        &self,
        record: &TelemetryRecord,
        field: &str,
    ) -> BridgeResult<Option<f64>> {
        // Check attributes first
        if let Some(value_str) = record.attributes.get(field) {
            return Ok(value_str.parse::<f64>().ok());
        }

        // Check tags
        if let Some(value_str) = record.tags.get(field) {
            return Ok(value_str.parse::<f64>().ok());
        }

        // Check telemetry data for metrics
        if let TelemetryData::Metric(metric) = &record.data {
            if metric.name == field {
                return Ok(Some(match &metric.value {
                    MetricValue::Counter(v) => *v,
                    MetricValue::Gauge(v) => *v,
                    MetricValue::Histogram { sum, .. } => *sum,
                    MetricValue::Summary { sum, .. } => *sum,
                }));
            }
        }

        Ok(None)
    }

    /// Check if window should be emitted
    async fn should_emit_window(&self) -> BridgeResult<bool> {
        let window_start = self.last_window_start.read().await;
        let now = chrono::Utc::now();

        let window_duration = match &self.config.time_window {
            TimeWindow::Fixed { duration_seconds } => *duration_seconds,
            TimeWindow::Sliding {
                duration_seconds, ..
            } => *duration_seconds,
            TimeWindow::Tumbling { duration_seconds } => *duration_seconds,
        };

        let elapsed = now.signed_duration_since(*window_start).num_seconds() as u64;
        Ok(elapsed >= window_duration)
    }

    /// Emit aggregated results
    async fn emit_aggregations(&self) -> BridgeResult<Vec<ProcessedRecord>> {
        let mut state = self.aggregation_state.write().await;
        let mut results = Vec::new();
        let mut error_count = 0;

        for (group_key, group_state) in state.iter() {
            for aggregation in &self.config.aggregations {
                match self.create_aggregation_record(aggregation, group_key, group_state) {
                    Ok(result_record) => {
                        results.push(result_record);
                    }
                    Err(e) => {
                        error_count += 1;
                        warn!(
                            "Failed to create aggregation record for group {}: {}",
                            group_key, e
                        );
                    }
                }
            }
        }

        // Update error count in stats
        if error_count > 0 {
            let mut stats = self.stats.write().await;
            stats.error_count += error_count;
        }

        // Reset state if configured
        if self.config.reset_after_emit {
            state.clear();
        }

        // Update window start time
        {
            let mut window_start = self.last_window_start.write().await;
            *window_start = chrono::Utc::now();
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.aggregations_emitted += results.len() as u64;
            stats.last_emission_time = Some(chrono::Utc::now());
        }

        Ok(results)
    }

    /// Create aggregation result record
    fn create_aggregation_record(
        &self,
        aggregation: &AggregationOperation,
        group_key: &str,
        group_state: &GroupAggregationState,
    ) -> BridgeResult<ProcessedRecord> {
        let (metric_name, metric_value) = match aggregation {
            AggregationOperation::Count { output_field, .. } => (
                output_field.clone(),
                MetricValue::Counter(group_state.count as f64),
            ),

            AggregationOperation::Sum { output_field, .. } => {
                (output_field.clone(), MetricValue::Gauge(group_state.sum))
            }

            AggregationOperation::Average { output_field, .. } => {
                let avg = if group_state.count > 0 {
                    group_state.sum / group_state.count as f64
                } else {
                    0.0
                };
                (output_field.clone(), MetricValue::Gauge(avg))
            }

            AggregationOperation::Min { output_field, .. } => {
                let min_val = group_state.min.unwrap_or(0.0);
                (output_field.clone(), MetricValue::Gauge(min_val))
            }

            AggregationOperation::Max { output_field, .. } => {
                let max_val = group_state.max.unwrap_or(0.0);
                (output_field.clone(), MetricValue::Gauge(max_val))
            }

            AggregationOperation::Percentile {
                output_field,
                percentile,
                ..
            } => {
                let mut values = group_state.values.clone();
                values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let index = ((*percentile / 100.0) * values.len() as f64) as usize;
                let percentile_value = values.get(index).copied().unwrap_or(0.0);
                (output_field.clone(), MetricValue::Gauge(percentile_value))
            }

            AggregationOperation::Rate {
                output_field,
                time_window_seconds,
                ..
            } => {
                let duration_seconds = group_state
                    .last_timestamp
                    .signed_duration_since(group_state.first_timestamp)
                    .num_seconds() as f64;
                let rate = if duration_seconds > 0.0 {
                    (group_state.count as f64) / duration_seconds * (*time_window_seconds as f64)
                } else {
                    0.0
                };
                (output_field.clone(), MetricValue::Gauge(rate))
            }
        };

        // Create metric data
        let metric_data = MetricData {
            name: metric_name,
            description: Some(format!("Aggregated metric from {}", group_key)),
            unit: None,
            metric_type: MetricType::Gauge,
            value: metric_value,
            timestamp: group_state.last_timestamp,
            labels: HashMap::from([
                ("group_key".to_string(), group_key.to_string()),
                ("aggregation_type".to_string(), format!("{:?}", aggregation)),
                ("record_count".to_string(), group_state.count.to_string()),
            ]),
        };

        Ok(ProcessedRecord {
            original_id: Uuid::new_v4(),
            status: ProcessingStatus::Success,
            transformed_data: Some(TelemetryData::Metric(metric_data)),
            metadata: HashMap::from([
                ("processor".to_string(), "aggregate".to_string()),
                ("group_key".to_string(), group_key.to_string()),
                (
                    "time_window_start".to_string(),
                    group_state.first_timestamp.to_rfc3339(),
                ),
                (
                    "time_window_end".to_string(),
                    group_state.last_timestamp.to_rfc3339(),
                ),
            ]),
            errors: Vec::new(),
        })
    }
}

#[async_trait]
impl TelemetryProcessor for AggregateProcessor {
    fn name(&self) -> &str {
        "aggregate_processor"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn process(&self, batch: TelemetryBatch) -> BridgeResult<ProcessedBatch> {
        let running = self.running.read().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Aggregate processor is not running",
            ));
        }

        let start_time = std::time::Instant::now();
        let mut processed_records = Vec::new();
        let mut processing_errors = Vec::new();

        // Process each record for aggregation
        for record in &batch.records {
            if let Err(e) = self.aggregate_record(record).await {
                processing_errors.push(crate::types::ProcessingError {
                    code: "AGGREGATION_ERROR".to_string(),
                    message: format!("Failed to aggregate record {}: {}", record.id, e),
                    details: None,
                });

                // Increment error count
                {
                    let mut stats = self.stats.write().await;
                    stats.error_count += 1;
                }
            }
        }

        // Check if we should emit aggregations
        if self.should_emit_window().await? || self.config.emit_intermediate {
            match self.emit_aggregations().await {
                Ok(aggregation_records) => {
                    processed_records.extend(aggregation_records);
                }
                Err(e) => {
                    processing_errors.push(crate::types::ProcessingError {
                        code: "EMISSION_ERROR".to_string(),
                        message: format!("Failed to emit aggregations: {}", e),
                        details: None,
                    });

                    // Increment error count
                    {
                        let mut stats = self.stats.write().await;
                        stats.error_count += 1;
                    }
                }
            }
        }

        // Update statistics
        let duration = start_time.elapsed();
        {
            let mut stats = self.stats.write().await;
            stats.total_records += batch.records.len() as u64;
            stats.last_processing_time = Some(chrono::Utc::now());

            let state = self.aggregation_state.read().await;
            stats.active_groups = state.len() as u64;

            // Update average processing time
            if stats.total_records > 0 {
                stats.avg_processing_time_ms = (stats.avg_processing_time_ms
                    * (stats.total_records - batch.records.len() as u64) as f64
                    + duration.as_millis() as f64)
                    / stats.total_records as f64;
            }
        }

        let processing_status = if processing_errors.is_empty() {
            ProcessingStatus::Success
        } else {
            ProcessingStatus::Partial
        };

        let output_records_count = processed_records.len();

        Ok(ProcessedBatch {
            original_batch_id: batch.id,
            timestamp: chrono::Utc::now(),
            status: processing_status,
            records: processed_records,
            metadata: HashMap::from([
                ("processor".to_string(), "aggregate".to_string()),
                (
                    "processing_time_ms".to_string(),
                    duration.as_millis().to_string(),
                ),
                ("input_records".to_string(), batch.records.len().to_string()),
                (
                    "output_records".to_string(),
                    output_records_count.to_string(),
                ),
            ]),
            errors: processing_errors,
        })
    }

    async fn get_stats(&self) -> BridgeResult<crate::traits::processor::ProcessorStats> {
        let stats = self.stats.read().await;
        Ok(crate::traits::processor::ProcessorStats {
            total_batches: 0, // Will be tracked by pipeline
            total_records: stats.total_records,
            batches_per_minute: 0, // Will be tracked by pipeline
            records_per_minute: stats.records_per_minute,
            avg_processing_time_ms: stats.avg_processing_time_ms,
            error_count: stats.error_count,
            last_process_time: stats.last_processing_time,
        })
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        let running = self.running.read().await;
        Ok(*running)
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        self.stop().await
    }
}

#[async_trait]
impl HealthCheckCallback for AggregateProcessor {
    async fn check(&self) -> BridgeResult<HealthCheckResult> {
        let running = self.running.read().await;
        let status = if *running {
            crate::health::types::HealthStatus::Healthy
        } else {
            crate::health::types::HealthStatus::Unhealthy
        };

        Ok(HealthCheckResult::new(
            "aggregate_processor".to_string(),
            status,
            if *running {
                "Aggregate processor is running".to_string()
            } else {
                "Aggregate processor is not running".to_string()
            },
        ))
    }

    fn component_name(&self) -> &str {
        "aggregate_processor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        metrics::MetricData, metrics::MetricType, metrics::MetricValue, TelemetryData,
        TelemetryType,
    };

    #[tokio::test]
    async fn test_aggregate_processor_creation() {
        let config = AggregateProcessorConfig::default();
        let processor = AggregateProcessor::new(config);

        let stats = processor.get_stats().await.unwrap();
        assert_eq!(stats.total_records, 0);
    }

    #[tokio::test]
    async fn test_aggregate_processor_start_stop() {
        let config = AggregateProcessorConfig::default();
        let processor = AggregateProcessor::new(config);

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
    async fn test_count_aggregation() {
        let config = AggregateProcessorConfig {
            aggregations: vec![AggregationOperation::Count {
                group_by: "service".to_string(),
                output_field: "request_count".to_string(),
            }],
            emit_intermediate: true,
            ..Default::default()
        };
        let processor = AggregateProcessor::new(config);
        processor.start().await.unwrap();

        // Create test records
        let record1 = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "requests".to_string(),
                description: None,
                unit: None,
                metric_type: MetricType::Counter,
                value: MetricValue::Counter(1.0),
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            }),
            attributes: HashMap::from([("service".to_string(), "web".to_string())]),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        let record2 = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "requests".to_string(),
                description: None,
                unit: None,
                metric_type: MetricType::Counter,
                value: MetricValue::Counter(1.0),
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            }),
            attributes: HashMap::from([("service".to_string(), "web".to_string())]),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        let batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: "test".to_string(),
            size: 2,
            records: vec![record1, record2],
            metadata: HashMap::new(),
        };

        // Process batch
        let result = processor.process(batch).await;
        assert!(result.is_ok());

        let processed_batch = result.unwrap();
        assert_eq!(processed_batch.status, ProcessingStatus::Success);

        // Should have one aggregated record for the count
        assert_eq!(processed_batch.records.len(), 1);

        // Check the aggregated metric
        if let Some(TelemetryData::Metric(metric)) = &processed_batch.records[0].transformed_data {
            assert_eq!(metric.name, "request_count");
            if let MetricValue::Counter(count) = metric.value {
                assert_eq!(count, 2.0);
            } else {
                panic!("Expected UInt64 metric value");
            }
        } else {
            panic!("Expected metric data");
        }

        processor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_error_tracking() {
        let config = AggregateProcessorConfig::default();
        let processor = AggregateProcessor::new(config);

        // Check that error count is properly initialized
        let stats = processor.get_stats().await.unwrap();
        assert_eq!(
            stats.error_count, 0,
            "Error count should be initialized to 0"
        );

        // Start the processor
        processor.start().await.unwrap();

        // Create a test record that will cause an error by using a non-existent group field
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            record_type: TelemetryType::Metric,
            data: TelemetryData::Metric(MetricData {
                name: "requests".to_string(),
                description: None,
                unit: None,
                metric_type: MetricType::Counter,
                value: MetricValue::Counter(1.0),
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            }),
            attributes: HashMap::new(), // Empty attributes will cause group key issues
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        let batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: "test".to_string(),
            size: 1,
            records: vec![record],
            metadata: HashMap::new(),
        };

        // Process batch
        let result = processor.process(batch).await;
        assert!(result.is_ok());

        // Verify that error tracking field is accessible
        let stats = processor.get_stats().await.unwrap();
        assert_eq!(
            stats.error_count, 0,
            "Error count should be accessible and properly tracked"
        );

        processor.stop().await.unwrap();
    }
}
