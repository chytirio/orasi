//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup@gmail.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Filter processor implementation
//!
//! This module provides a filter processor for streaming data filtering.

use async_trait::async_trait;
use bridge_core::{
    traits::{DataStream, StreamProcessor as BridgeStreamProcessor, StreamProcessorStats},
    types::{TelemetryData, TelemetryRecord},
    BridgeResult, TelemetryBatch,
};
use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::ProcessorConfig;

/// Filter processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterProcessorConfig {
    /// Processor name
    pub name: String,

    /// Processor version
    pub version: String,

    /// Filter rules
    pub filter_rules: Vec<FilterRule>,

    /// Filter mode
    pub filter_mode: FilterMode,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Filter rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    /// Rule name
    pub name: String,

    /// Field to filter on
    pub field: String,

    /// Filter operator
    pub operator: FilterOperator,

    /// Filter value
    pub value: String,

    /// Whether rule is enabled
    pub enabled: bool,
}

/// Filter operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    NotContains,
    Regex,
    In,
    NotIn,
}

/// Filter mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterMode {
    Include,
    Exclude,
}

impl FilterProcessorConfig {
    /// Create new filter processor configuration
    pub fn new(filter_rules: Vec<FilterRule>, filter_mode: FilterMode) -> Self {
        Self {
            name: "filter".to_string(),
            version: "1.0.0".to_string(),
            filter_rules,
            filter_mode,
            additional_config: HashMap::new(),
        }
    }
}

#[async_trait]
impl ProcessorConfig for FilterProcessorConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        if self.filter_rules.is_empty() {
            return Err(bridge_core::BridgeError::configuration(
                "Filter rules cannot be empty".to_string(),
            ));
        }

        // Validate regex patterns
        for rule in &self.filter_rules {
            if let FilterOperator::Regex = rule.operator {
                if let Err(e) = Regex::new(&rule.value) {
                    return Err(bridge_core::BridgeError::configuration(format!(
                        "Invalid regex pattern in rule '{}': {}",
                        rule.name, e
                    )));
                }
            }
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Filter processor implementation
pub struct FilterProcessor {
    config: FilterProcessorConfig,
    compiled_regex: HashMap<String, Regex>,
    stats: Arc<RwLock<StreamProcessorStats>>,
}

impl FilterProcessor {
    /// Create new filter processor
    pub async fn new(config: &dyn ProcessorConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<FilterProcessorConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration(
                    "Invalid filter processor configuration".to_string(),
                )
            })?
            .clone();

        config.validate().await?;

        let mut compiled_regex = HashMap::new();

        // Pre-compile regex patterns
        for rule in &config.filter_rules {
            if let FilterOperator::Regex = rule.operator {
                match Regex::new(&rule.value) {
                    Ok(regex) => {
                        compiled_regex.insert(rule.name.clone(), regex);
                    }
                    Err(e) => {
                        return Err(bridge_core::BridgeError::configuration(format!(
                            "Invalid regex pattern in rule '{}': {}",
                            rule.name, e
                        )));
                    }
                }
            }
        }

        let stats = StreamProcessorStats {
            total_records: 0,
            records_per_minute: 0,
            avg_processing_time_ms: 0.0,
            error_count: 0,
            last_process_time: None,
        };

        Ok(Self {
            config,
            compiled_regex,
            stats: Arc::new(RwLock::new(stats)),
        })
    }

    /// Process data stream with filtering logic
    async fn process_data_stream(&self, input: DataStream) -> BridgeResult<DataStream> {
        let start_time = std::time::Instant::now();

        info!("Processing data stream with filters: {}", input.stream_id);

        // Deserialize the data stream to TelemetryBatch
        let batch: TelemetryBatch = serde_json::from_slice(&input.data).map_err(|e| {
            bridge_core::BridgeError::internal(format!("Failed to deserialize data stream: {}", e))
        })?;

        // Apply filter rules
        let filtered_records: Vec<TelemetryRecord> = batch
            .records
            .into_iter()
            .filter(|record| self.should_include_record(record))
            .collect();

        // Create filtered batch
        let filtered_batch = TelemetryBatch {
            id: batch.id,
            timestamp: batch.timestamp,
            source: batch.source,
            size: filtered_records.len(),
            records: filtered_records,
            metadata: batch.metadata,
        };

        // Serialize filtered batch back to stream
        let filtered_data = serde_json::to_vec(&filtered_batch).map_err(|e| {
            bridge_core::BridgeError::internal(format!("Failed to serialize filtered batch: {}", e))
        })?;

        // Update metadata
        let mut output_metadata = input.metadata.clone();
        output_metadata.insert("filtered_by".to_string(), self.config.name.clone());
        output_metadata.insert("filtered_at".to_string(), Utc::now().to_rfc3339());
        output_metadata.insert(
            "filter_rules_applied".to_string(),
            self.config.filter_rules.len().to_string(),
        );
        output_metadata.insert("original_size".to_string(), batch.size.to_string());
        output_metadata.insert("filtered_size".to_string(), filtered_batch.size.to_string());

        let processed_stream = DataStream {
            stream_id: format!("filtered_{}", input.stream_id),
            data: filtered_data,
            metadata: output_metadata,
            timestamp: Utc::now(),
        };

        let processing_time = start_time.elapsed().as_millis() as f64;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_records += batch.size as u64;
            stats.avg_processing_time_ms = (stats.avg_processing_time_ms + processing_time) / 2.0;
            stats.last_process_time = Some(Utc::now());
        }

        info!(
            "Data stream filtered: {} -> {} records in {}ms",
            batch.size, filtered_batch.size, processing_time
        );

        Ok(processed_stream)
    }

    /// Determine if a record should be included based on filter rules
    fn should_include_record(&self, record: &TelemetryRecord) -> bool {
        for rule in &self.config.filter_rules {
            if !rule.enabled {
                continue;
            }

            let field_value = self.get_field_value(record, &rule.field);
            let rule_matches = self.evaluate_rule(rule, &field_value);

            match self.config.filter_mode {
                FilterMode::Include => {
                    // Include if ANY rule matches
                    if rule_matches {
                        return true;
                    }
                }
                FilterMode::Exclude => {
                    // Exclude if ANY rule matches
                    if rule_matches {
                        return false;
                    }
                }
            }
        }

        // Default behavior based on filter mode
        match self.config.filter_mode {
            FilterMode::Include => false, // Must match at least one rule to be included
            FilterMode::Exclude => true,  // Must not match any rule to be included
        }
    }

    /// Get field value from a telemetry record
    fn get_field_value(&self, record: &TelemetryRecord, field_path: &str) -> Option<String> {
        // Simple field path resolution - in practice this would be more sophisticated
        match field_path {
            "id" => Some(record.id.to_string()),
            "timestamp" => Some(record.timestamp.to_rfc3339()),
            "record_type" => Some(format!("{:?}", record.record_type)),
            _ => {
                // Check attributes
                if let Some(value) = record.attributes.get(field_path) {
                    Some(value.clone())
                } else if let Some(value) = record.tags.get(field_path) {
                    Some(value.clone())
                } else {
                    // Check metric data if this is a metric record
                    if let TelemetryData::Metric(metric_data) = &record.data {
                        match field_path {
                            "metric.name" => Some(metric_data.name.clone()),
                            "metric.description" => {
                                Some(metric_data.description.clone().unwrap_or_default())
                            }
                            "metric.unit" => Some(metric_data.unit.clone().unwrap_or_default()),
                            "metric.value" => Some(format!("{:?}", metric_data.value)),
                            _ => {
                                // Check metric labels
                                metric_data.labels.get(field_path).cloned()
                            }
                        }
                    } else {
                        None
                    }
                }
            }
        }
    }

    /// Evaluate a filter rule against a field value
    fn evaluate_rule(&self, rule: &FilterRule, field_value: &Option<String>) -> bool {
        let field_value = match field_value {
            Some(v) => v,
            None => return false, // Field not found, rule doesn't match
        };

        match rule.operator {
            FilterOperator::Equals => field_value == &rule.value,
            FilterOperator::NotEquals => field_value != &rule.value,
            FilterOperator::Contains => field_value.contains(&rule.value),
            FilterOperator::NotContains => !field_value.contains(&rule.value),
            FilterOperator::Regex => {
                if let Some(regex) = self.compiled_regex.get(&rule.name) {
                    regex.is_match(field_value)
                } else {
                    false
                }
            }
            FilterOperator::GreaterThan => {
                if let (Ok(field_num), Ok(rule_num)) =
                    (field_value.parse::<f64>(), rule.value.parse::<f64>())
                {
                    field_num > rule_num
                } else {
                    false
                }
            }
            FilterOperator::LessThan => {
                if let (Ok(field_num), Ok(rule_num)) =
                    (field_value.parse::<f64>(), rule.value.parse::<f64>())
                {
                    field_num < rule_num
                } else {
                    false
                }
            }
            FilterOperator::GreaterThanOrEqual => {
                if let (Ok(field_num), Ok(rule_num)) =
                    (field_value.parse::<f64>(), rule.value.parse::<f64>())
                {
                    field_num >= rule_num
                } else {
                    false
                }
            }
            FilterOperator::LessThanOrEqual => {
                if let (Ok(field_num), Ok(rule_num)) =
                    (field_value.parse::<f64>(), rule.value.parse::<f64>())
                {
                    field_num <= rule_num
                } else {
                    false
                }
            }
            FilterOperator::In => {
                // Simple comma-separated list for "in" operator
                let values: Vec<&str> = rule.value.split(',').map(|s| s.trim()).collect();
                values.contains(&field_value.as_str())
            }
            FilterOperator::NotIn => {
                // Simple comma-separated list for "not in" operator
                let values: Vec<&str> = rule.value.split(',').map(|s| s.trim()).collect();
                !values.contains(&field_value.as_str())
            }
        }
    }
}

#[async_trait]
impl BridgeStreamProcessor for FilterProcessor {
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
        info!("Shutting down filter processor: {}", self.config.name);
        Ok(())
    }
}
