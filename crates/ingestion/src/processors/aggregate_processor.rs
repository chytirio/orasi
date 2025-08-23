//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Aggregate processor implementation
//!
//! This module provides an aggregate processor for aggregating telemetry data
//! using various aggregation functions and windowing strategies.

use async_trait::async_trait;
use bridge_core::{
    traits::ProcessorStats,
    types::{
        ProcessedBatch, ProcessedRecord, ProcessingStatus, TelemetryBatch, TelemetryData,
        TelemetryRecord,
    },
    BridgeResult,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use super::{BaseProcessor, ProcessorConfig};

/// Aggregate processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateProcessorConfig {
    /// Processor name
    pub name: String,

    /// Processor version
    pub version: String,

    /// Aggregation rules
    pub aggregation_rules: Vec<AggregationRule>,

    /// Window configuration
    pub window_config: WindowConfig,

    /// Enable state persistence
    pub enable_state_persistence: bool,

    /// State persistence path
    pub state_persistence_path: Option<String>,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Aggregation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationRule {
    /// Rule name
    pub name: String,

    /// Rule description
    pub description: Option<String>,

    /// Group by fields
    pub group_by_fields: Vec<String>,

    /// Aggregation functions
    pub aggregation_functions: Vec<AggregationFunction>,

    /// Filter condition (optional)
    pub filter_condition: Option<String>,

    /// Rule enabled
    pub enabled: bool,
}

/// Aggregation function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationFunction {
    /// Function name
    pub name: String,

    /// Source field
    pub source_field: String,

    /// Target field
    pub target_field: String,

    /// Function type
    pub function_type: AggregationFunctionType,

    /// Function parameters
    pub parameters: HashMap<String, String>,
}

/// Aggregation function type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationFunctionType {
    Count,
    Sum,
    Average,
    Min,
    Max,
    Median,
    Percentile,
    Variance,
    StandardDeviation,
    Custom,
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Window type
    pub window_type: WindowType,

    /// Window size in milliseconds
    pub window_size_ms: u64,

    /// Slide interval in milliseconds
    pub slide_interval_ms: u64,

    /// Watermark delay in milliseconds
    pub watermark_delay_ms: u64,

    /// Allowed lateness in milliseconds
    pub allowed_lateness_ms: u64,
}

/// Window type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowType {
    Tumbling,
    Sliding,
    Session,
}

/// Aggregation state
#[derive(Debug, Clone)]
struct AggregationState {
    /// Group key
    group_key: String,

    /// Window start time
    window_start: DateTime<Utc>,

    /// Window end time
    window_end: DateTime<Utc>,

    /// Aggregated values
    aggregated_values: HashMap<String, AggregatedValue>,

    /// Record count
    record_count: u64,

    /// Last update time
    last_update: DateTime<Utc>,
}

/// Aggregated value
#[derive(Debug, Clone)]
struct AggregatedValue {
    /// Value type
    value_type: AggregationFunctionType,

    /// Current value
    value: f64,

    /// Count of values
    count: u64,

    /// All values for percentile calculations
    values: VecDeque<f64>,

    /// Parameters
    parameters: HashMap<String, String>,
}

/// Aggregate processor
pub struct AggregateProcessor {
    config: AggregateProcessorConfig,
    stats: Arc<RwLock<ProcessorStats>>,
    state_store: Arc<RwLock<HashMap<String, AggregationState>>>,
}

impl AggregateProcessorConfig {
    /// Create new aggregate processor configuration
    pub fn new(aggregation_rules: Vec<AggregationRule>, window_config: WindowConfig) -> Self {
        Self {
            name: "aggregate".to_string(),
            version: "1.0.0".to_string(),
            aggregation_rules,
            window_config,
            enable_state_persistence: false,
            state_persistence_path: None,
            additional_config: HashMap::new(),
        }
    }
}

#[async_trait]
impl ProcessorConfig for AggregateProcessorConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        // Validate aggregation rules
        for rule in &self.aggregation_rules {
            if rule.name.is_empty() {
                return Err(bridge_core::BridgeError::configuration(
                    "Aggregation rule name cannot be empty",
                ));
            }

            if rule.group_by_fields.is_empty() {
                return Err(bridge_core::BridgeError::configuration(
                    "Aggregation rule must have at least one group by field",
                ));
            }

            if rule.aggregation_functions.is_empty() {
                return Err(bridge_core::BridgeError::configuration(
                    "Aggregation rule must have at least one aggregation function",
                ));
            }

            for func in &rule.aggregation_functions {
                if func.name.is_empty() {
                    return Err(bridge_core::BridgeError::configuration(
                        "Aggregation function name cannot be empty",
                    ));
                }

                if func.source_field.is_empty() {
                    return Err(bridge_core::BridgeError::configuration(
                        "Aggregation function source field cannot be empty",
                    ));
                }

                if func.target_field.is_empty() {
                    return Err(bridge_core::BridgeError::configuration(
                        "Aggregation function target field cannot be empty",
                    ));
                }
            }
        }

        // Validate window configuration
        if self.window_config.window_size_ms == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Window size cannot be zero",
            ));
        }

        if self.window_config.slide_interval_ms == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Slide interval cannot be zero",
            ));
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl AggregateProcessor {
    /// Create new aggregate processor
    pub async fn new(config: &dyn ProcessorConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<AggregateProcessorConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration("Invalid aggregate processor configuration")
            })?
            .clone();

        config.validate().await?;

        let stats = ProcessorStats {
            total_batches: 0,
            total_records: 0,
            batches_per_minute: 0,
            records_per_minute: 0,
            avg_processing_time_ms: 0.0,
            error_count: 0,
            last_process_time: None,
        };

        Ok(Self {
            config,
            stats: Arc::new(RwLock::new(stats)),
            state_store: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Process telemetry batch with aggregation
    async fn process_batch(&self, batch: TelemetryBatch) -> BridgeResult<ProcessedBatch> {
        let start_time = std::time::Instant::now();

        info!(
            "Processing batch with aggregations: {} records",
            batch.records.len()
        );

        let mut processed_records = Vec::new();

        // Process each record
        for record in batch.records {
            let processed_record = self.process_record(record).await?;
            processed_records.push(processed_record);
        }

        // Generate aggregated records
        let aggregated_records = self.generate_aggregated_records().await?;
        processed_records.extend(aggregated_records);

        let processing_time = start_time.elapsed().as_millis() as f64;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_records += batch.size as u64;
            stats.avg_processing_time_ms = (stats.avg_processing_time_ms + processing_time) / 2.0;
            stats.last_process_time = Some(Utc::now());
        }

        info!(
            "Batch processed with aggregations: {} -> {} records in {}ms",
            batch.size,
            processed_records.len(),
            processing_time
        );

        Ok(ProcessedBatch {
            original_batch_id: batch.id,
            timestamp: Utc::now(),
            status: ProcessingStatus::Success,
            records: processed_records,
            metadata: batch.metadata,
            errors: Vec::new(),
        })
    }

    /// Process individual record
    async fn process_record(&self, record: TelemetryRecord) -> BridgeResult<ProcessedRecord> {
        // Apply aggregation rules
        for rule in &self.config.aggregation_rules {
            if !rule.enabled {
                continue;
            }

            // Check filter condition
            if let Some(condition) = &rule.filter_condition {
                if !self.evaluate_condition(condition, &record) {
                    continue;
                }
            }

            // Generate group key
            let group_key = self.generate_group_key(rule, &record);

            // Update aggregation state
            self.update_aggregation_state(rule, &group_key, &record)
                .await?;
        }

        // Convert to processed record
        Ok(ProcessedRecord {
            original_id: record.id,
            status: ProcessingStatus::Success,
            transformed_data: Some(record.data),
            metadata: HashMap::new(),
            errors: Vec::new(),
        })
    }

    /// Generate group key for aggregation
    fn generate_group_key(&self, rule: &AggregationRule, record: &TelemetryRecord) -> String {
        let mut key_parts = Vec::new();

        for field in &rule.group_by_fields {
            let value = self.get_field_value(record, field);
            key_parts.push(format!("{}={}", field, value));
        }

        key_parts.join("|")
    }

    /// Get field value from record
    fn get_field_value(&self, record: &TelemetryRecord, field: &str) -> String {
        // Handle nested field paths
        let field_parts: Vec<&str> = field.split('.').collect();

        match field_parts.as_slice() {
            ["id"] => record.id.to_string(),
            ["timestamp"] => record.timestamp.to_rfc3339(),
            ["record_type"] => format!("{:?}", record.record_type),
            ["attributes", key] => record.attributes.get(*key).cloned().unwrap_or_default(),
            ["tags", key] => record.tags.get(*key).cloned().unwrap_or_default(),
            ["data", "metric", "name"] => {
                if let TelemetryData::Metric(metric) = &record.data {
                    metric.name.clone()
                } else {
                    String::new()
                }
            }
            ["data", "log", "message"] => {
                if let TelemetryData::Log(log) = &record.data {
                    log.message.clone()
                } else {
                    String::new()
                }
            }
            _ => {
                // Try to find in attributes first, then tags
                if let Some(value) = record.attributes.get(field) {
                    value.clone()
                } else if let Some(value) = record.tags.get(field) {
                    value.clone()
                } else {
                    String::new()
                }
            }
        }
    }

    /// Update aggregation state
    async fn update_aggregation_state(
        &self,
        rule: &AggregationRule,
        group_key: &str,
        record: &TelemetryRecord,
    ) -> BridgeResult<()> {
        let mut state_store = self.state_store.write().await;

        // Get or create aggregation state
        let state = state_store.entry(group_key.to_string()).or_insert_with(|| {
            let now = Utc::now();
            let window_start = self.calculate_window_start(now);
            let window_end = self.calculate_window_end(window_start);

            AggregationState {
                group_key: group_key.to_string(),
                window_start,
                window_end,
                aggregated_values: HashMap::new(),
                record_count: 0,
                last_update: now,
            }
        });

        // Update aggregated values
        for func in &rule.aggregation_functions {
            let source_value = self.get_numeric_value(record, &func.source_field);
            if let Some(value) = source_value {
                self.update_aggregated_value(state, func, value).await?;
            }
        }

        state.record_count += 1;
        state.last_update = Utc::now();

        Ok(())
    }

    /// Update aggregated value
    async fn update_aggregated_value(
        &self,
        state: &mut AggregationState,
        func: &AggregationFunction,
        value: f64,
    ) -> BridgeResult<()> {
        let aggregated_value = state
            .aggregated_values
            .entry(func.target_field.clone())
            .or_insert_with(|| AggregatedValue {
                value_type: func.function_type.clone(),
                value: 0.0,
                count: 0,
                values: VecDeque::new(),
                parameters: func.parameters.clone(),
            });

        match func.function_type {
            AggregationFunctionType::Count => {
                aggregated_value.value += 1.0;
                aggregated_value.count += 1;
            }
            AggregationFunctionType::Sum => {
                aggregated_value.value += value;
                aggregated_value.count += 1;
            }
            AggregationFunctionType::Average => {
                aggregated_value.value = (aggregated_value.value * aggregated_value.count as f64
                    + value)
                    / (aggregated_value.count + 1) as f64;
                aggregated_value.count += 1;
            }
            AggregationFunctionType::Min => {
                if aggregated_value.count == 0 || value < aggregated_value.value {
                    aggregated_value.value = value;
                }
                aggregated_value.count += 1;
            }
            AggregationFunctionType::Max => {
                if aggregated_value.count == 0 || value > aggregated_value.value {
                    aggregated_value.value = value;
                }
                aggregated_value.count += 1;
            }
            AggregationFunctionType::Median | AggregationFunctionType::Percentile => {
                aggregated_value.values.push_back(value);
                aggregated_value.count += 1;

                // Keep only recent values for memory efficiency
                if aggregated_value.values.len() > 1000 {
                    aggregated_value.values.pop_front();
                }
            }
            AggregationFunctionType::Variance | AggregationFunctionType::StandardDeviation => {
                aggregated_value.values.push_back(value);
                aggregated_value.count += 1;

                // Keep only recent values for memory efficiency
                if aggregated_value.values.len() > 1000 {
                    aggregated_value.values.pop_front();
                }
            }
            AggregationFunctionType::Custom => {
                // Custom aggregation logic
                aggregated_value.value += value;
                aggregated_value.count += 1;
            }
        }

        Ok(())
    }

    /// Get numeric value from record
    fn get_numeric_value(&self, record: &TelemetryRecord, field: &str) -> Option<f64> {
        let value_str = self.get_field_value(record, field);
        value_str.parse::<f64>().ok()
    }

    /// Calculate window start time
    fn calculate_window_start(&self, timestamp: DateTime<Utc>) -> DateTime<Utc> {
        match self.config.window_config.window_type {
            WindowType::Tumbling => {
                let window_size =
                    Duration::milliseconds(self.config.window_config.window_size_ms as i64);
                let window_start = timestamp
                    - Duration::milliseconds(
                        timestamp.timestamp_millis() % window_size.num_milliseconds(),
                    );
                window_start
            }
            WindowType::Sliding => {
                let slide_interval =
                    Duration::milliseconds(self.config.window_config.slide_interval_ms as i64);
                let window_start = timestamp
                    - Duration::milliseconds(
                        timestamp.timestamp_millis() % slide_interval.num_milliseconds(),
                    );
                window_start
            }
            WindowType::Session => {
                // Session windows start when the first record arrives
                timestamp
            }
        }
    }

    /// Calculate window end time
    fn calculate_window_end(&self, window_start: DateTime<Utc>) -> DateTime<Utc> {
        let window_size = Duration::milliseconds(self.config.window_config.window_size_ms as i64);
        window_start + window_size
    }

    /// Evaluate condition
    fn evaluate_condition(&self, condition: &str, record: &TelemetryRecord) -> bool {
        // Simple condition evaluation
        // In a full implementation, this would use a proper expression evaluator
        condition.contains("true") || condition.is_empty()
    }

    /// Generate aggregated records
    async fn generate_aggregated_records(&self) -> BridgeResult<Vec<ProcessedRecord>> {
        let mut aggregated_records = Vec::new();
        let mut state_store = self.state_store.write().await;

        // Process each aggregation state
        for (group_key, state) in state_store.iter_mut() {
            if self.should_emit_window(state) {
                let record = self.create_aggregated_record(state).await?;
                aggregated_records.push(record);
            }
        }

        // Clean up expired windows
        self.cleanup_expired_windows(&mut state_store).await;

        Ok(aggregated_records)
    }

    /// Check if window should be emitted
    fn should_emit_window(&self, state: &AggregationState) -> bool {
        let now = Utc::now();
        let watermark =
            now - Duration::milliseconds(self.config.window_config.watermark_delay_ms as i64);

        state.window_end <= watermark
    }

    /// Create aggregated record
    async fn create_aggregated_record(
        &self,
        state: &AggregationState,
    ) -> BridgeResult<ProcessedRecord> {
        let mut attributes: HashMap<String, String> = HashMap::new();
        let tags: HashMap<String, String> = HashMap::new();

        // Add group key information
        for part in state.group_key.split('|') {
            if let Some((key, value)) = part.split_once('=') {
                attributes.insert(format!("group_{}", key), value.to_string());
            }
        }

        // Add aggregation results
        for (field, aggregated_value) in &state.aggregated_values {
            let final_value = self.calculate_final_value(aggregated_value).await?;
            attributes.insert(field.clone(), final_value.to_string());
        }

        // Add window information
        attributes.insert("window_start".to_string(), state.window_start.to_rfc3339());
        attributes.insert("window_end".to_string(), state.window_end.to_rfc3339());
        attributes.insert("record_count".to_string(), state.record_count.to_string());

        Ok(ProcessedRecord {
            original_id: Uuid::new_v4(),
            status: ProcessingStatus::Success,
            transformed_data: Some(TelemetryData::Metric(bridge_core::types::MetricData {
                name: "aggregated_metric".to_string(),
                description: Some("Aggregated telemetry data".to_string()),
                unit: Some("count".to_string()),
                metric_type: bridge_core::types::MetricType::Gauge,
                value: bridge_core::types::MetricValue::Gauge(state.record_count as f64),
                labels: HashMap::new(),
                timestamp: state.window_end,
            })),
            metadata: HashMap::new(),
            errors: Vec::new(),
        })
    }

    /// Calculate final value for aggregation
    async fn calculate_final_value(&self, aggregated_value: &AggregatedValue) -> BridgeResult<f64> {
        match aggregated_value.value_type {
            AggregationFunctionType::Count => Ok(aggregated_value.value),
            AggregationFunctionType::Sum => Ok(aggregated_value.value),
            AggregationFunctionType::Average => Ok(aggregated_value.value),
            AggregationFunctionType::Min => Ok(aggregated_value.value),
            AggregationFunctionType::Max => Ok(aggregated_value.value),
            AggregationFunctionType::Median => {
                if aggregated_value.values.is_empty() {
                    return Ok(0.0);
                }
                let mut sorted_values: Vec<f64> = aggregated_value.values.iter().copied().collect();
                sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let mid = sorted_values.len() / 2;
                if sorted_values.len() % 2 == 0 {
                    Ok((sorted_values[mid - 1] + sorted_values[mid]) / 2.0)
                } else {
                    Ok(sorted_values[mid])
                }
            }
            AggregationFunctionType::Percentile => {
                if aggregated_value.values.is_empty() {
                    return Ok(0.0);
                }
                let percentile = aggregated_value
                    .parameters
                    .get("percentile")
                    .and_then(|p| p.parse::<f64>().ok())
                    .unwrap_or(95.0);

                let mut sorted_values: Vec<f64> = aggregated_value.values.iter().copied().collect();
                sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

                let index =
                    (percentile / 100.0 * (sorted_values.len() - 1) as f64).round() as usize;
                Ok(sorted_values[index.min(sorted_values.len() - 1)])
            }
            AggregationFunctionType::Variance => {
                if aggregated_value.values.len() < 2 {
                    return Ok(0.0);
                }
                let mean = aggregated_value.values.iter().sum::<f64>()
                    / aggregated_value.values.len() as f64;
                let variance = aggregated_value
                    .values
                    .iter()
                    .map(|x| (x - mean).powi(2))
                    .sum::<f64>()
                    / (aggregated_value.values.len() - 1) as f64;
                Ok(variance)
            }
            AggregationFunctionType::StandardDeviation => {
                if aggregated_value.values.len() < 2 {
                    return Ok(0.0);
                }
                let mean = aggregated_value.values.iter().sum::<f64>()
                    / aggregated_value.values.len() as f64;
                let variance = aggregated_value
                    .values
                    .iter()
                    .map(|x| (x - mean).powi(2))
                    .sum::<f64>()
                    / (aggregated_value.values.len() - 1) as f64;
                Ok(variance.sqrt())
            }
            AggregationFunctionType::Custom => Ok(aggregated_value.value),
        }
    }

    /// Cleanup expired windows
    async fn cleanup_expired_windows(&self, state_store: &mut HashMap<String, AggregationState>) {
        let now = Utc::now();
        let allowed_lateness =
            Duration::milliseconds(self.config.window_config.allowed_lateness_ms as i64);
        let cutoff_time = now - allowed_lateness;

        state_store.retain(|_, state| state.window_end > cutoff_time);
    }
}

#[async_trait]
impl bridge_core::traits::TelemetryProcessor for AggregateProcessor {
    async fn process(&self, batch: TelemetryBatch) -> BridgeResult<ProcessedBatch> {
        self.process_batch(batch).await
    }

    fn name(&self) -> &str {
        "aggregate_processor"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        Ok(true)
    }

    async fn get_stats(&self) -> BridgeResult<ProcessorStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Shutting down aggregate processor");
        Ok(())
    }
}
