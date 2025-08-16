//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Filter processor implementation
//!
//! This module provides a filter processor for filtering telemetry data
//! based on various criteria.

use async_trait::async_trait;
use bridge_core::{
    traits::ProcessorStats, types::{ProcessedRecord, TelemetryData, TelemetryRecord}, BridgeResult, ProcessedBatch, TelemetryBatch,
    TelemetryProcessor as BridgeTelemetryProcessor,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;
use regex::Regex;

use super::{BaseProcessor, ProcessorConfig};

/// Filter processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterProcessorConfig {
    /// Processor name
    pub name: String,

    /// Processor version
    pub version: String,

    /// Filter rules
    pub filter_rules: Vec<FilterRule>,

    /// Filter mode (include or exclude)
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

    /// Rule enabled
    pub enabled: bool,
}

/// Filter operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Regex,
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
        // Validate filter rules
        for rule in &self.filter_rules {
            if rule.field.is_empty() {
                return Err(bridge_core::BridgeError::configuration(
                    "Filter rule field cannot be empty",
                ));
            }

            if rule.name.is_empty() {
                return Err(bridge_core::BridgeError::configuration(
                    "Filter rule name cannot be empty",
                ));
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
    base: BaseProcessor,
    config: FilterProcessorConfig,
}

impl FilterProcessor {
    /// Create new filter processor
    pub async fn new(config: &dyn ProcessorConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<FilterProcessorConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration("Invalid filter processor configuration")
            })?
            .clone();

        config.validate().await?;

        let base = BaseProcessor::new(config.name.clone(), config.version.clone());

        Ok(Self { base, config })
    }

    /// Apply filter rules to a record
    fn apply_filters(&self, record: &TelemetryRecord) -> bool {
        let mut matches = true;

        for rule in &self.config.filter_rules {
            if !rule.enabled {
                continue;
            }

            let field_value = self.get_field_value(record, &rule.field);
            let rule_matches = self.evaluate_rule(&rule.operator, &field_value, &rule.value);

            matches = matches && rule_matches;
        }

        match self.config.filter_mode {
            FilterMode::Include => matches,
            FilterMode::Exclude => !matches,
        }
    }

    /// Get field value from record
    fn get_field_value(&self, record: &TelemetryRecord, field: &str) -> String {
        // Implement field value extraction
        // Extract values from record attributes, tags, etc.
        
        // Handle nested field paths (e.g., "attributes.service.name")
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
            ["data", "event", "name"] => {
                if let TelemetryData::Event(event) = &record.data {
                    event.name.clone()
                } else {
                    String::new()
                }
            }
            ["data", "trace", "trace_id"] => {
                if let TelemetryData::Trace(trace) = &record.data {
                    trace.trace_id.clone()
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

    /// Evaluate filter rule
    fn evaluate_rule(
        &self,
        operator: &FilterOperator,
        field_value: &str,
        rule_value: &str,
    ) -> bool {
        match operator {
            FilterOperator::Equals => field_value == rule_value,
            FilterOperator::NotEquals => field_value != rule_value,
            FilterOperator::Contains => field_value.contains(rule_value),
            FilterOperator::NotContains => !field_value.contains(rule_value),
            FilterOperator::GreaterThan => {
                if let (Ok(field_num), Ok(rule_num)) =
                    (field_value.parse::<f64>(), rule_value.parse::<f64>())
                {
                    field_num > rule_num
                } else {
                    field_value > rule_value
                }
            }
            FilterOperator::LessThan => {
                if let (Ok(field_num), Ok(rule_num)) =
                    (field_value.parse::<f64>(), rule_value.parse::<f64>())
                {
                    field_num < rule_num
                } else {
                    field_value < rule_value
                }
            }
            FilterOperator::GreaterThanOrEqual => {
                if let (Ok(field_num), Ok(rule_num)) =
                    (field_value.parse::<f64>(), rule_value.parse::<f64>())
                {
                    field_num >= rule_num
                } else {
                    field_value >= rule_value
                }
            }
            FilterOperator::LessThanOrEqual => {
                if let (Ok(field_num), Ok(rule_num)) =
                    (field_value.parse::<f64>(), rule_value.parse::<f64>())
                {
                    field_num <= rule_num
                } else {
                    field_value <= rule_value
                }
            }
            FilterOperator::Regex => {
                // Implement regex matching using regex crate
                match Regex::new(rule_value) {
                    Ok(regex) => regex.is_match(field_value),
                    Err(_) => {
                        // If regex compilation fails, fall back to simple contains
                        warn!("Invalid regex pattern '{}', falling back to contains", rule_value);
                        field_value.contains(rule_value)
                    }
                }
            }
        }
    }
}

#[async_trait]
impl BridgeTelemetryProcessor for FilterProcessor {
    async fn process(&self, batch: TelemetryBatch) -> BridgeResult<ProcessedBatch> {
        let start_time = std::time::Instant::now();

        // Filter records based on rules
        let filtered_records: Vec<TelemetryRecord> = batch
            .records
            .into_iter()
            .filter(|record| self.apply_filters(record))
            .collect();

        let filtered_batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: batch.source,
            size: filtered_records.len(),
            records: filtered_records,
            metadata: batch.metadata,
        };

        let processing_time = start_time.elapsed();
        let processing_time_ms = processing_time.as_millis() as f64;

        // Update statistics
        self.base
            .update_stats(1, filtered_batch.size, processing_time_ms)
            .await;

        Ok(ProcessedBatch {
            original_batch_id: batch.id,
            timestamp: Utc::now(),
            status: bridge_core::types::ProcessingStatus::Success,
            records: filtered_batch
                .records
                .into_iter()
                .map(|r| ProcessedRecord {
                    original_id: r.id,
                    status: bridge_core::types::ProcessingStatus::Success,
                    transformed_data: Some(r.data),
                    metadata: r.attributes,
                    errors: vec![],
                })
                .collect(),
            metadata: filtered_batch.metadata,
            errors: vec![],
        })
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn version(&self) -> &str {
        &self.base.version
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        Ok(true)
    }

    async fn get_stats(&self) -> BridgeResult<ProcessorStats> {
        let stats = self.base.stats.read().await;
        Ok(stats.clone())
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        info!("Filter processor shutdown completed");
        Ok(())
    }
}

impl FilterProcessor {
    /// Get filter configuration
    pub fn get_config(&self) -> &FilterProcessorConfig {
        &self.config
    }

    /// Add filter rule
    pub fn add_filter_rule(&mut self, rule: FilterRule) {
        self.config.filter_rules.push(rule);
    }

    /// Remove filter rule by name
    pub fn remove_filter_rule(&mut self, rule_name: &str) {
        self.config
            .filter_rules
            .retain(|rule| rule.name != rule_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_core::types::{MetricData, MetricType, MetricValue, TelemetryData, TelemetryType};

    #[tokio::test]
    async fn test_filter_processor_processed_record_conversion() {
        // Create a filter processor with a simple rule
        let filter_rules = vec![FilterRule {
            name: "test_rule".to_string(),
            field: "name".to_string(),
            operator: FilterOperator::Contains,
            value: "test".to_string(),
            enabled: true,
        }];

        let config = FilterProcessorConfig::new(filter_rules, FilterMode::Include);
        let processor = FilterProcessor::new(&config).await.unwrap();

        // Create test records
        let records = vec![
            TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name: "test_metric".to_string(),
                    description: Some("Test metric".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: MetricType::Gauge,
                    value: MetricValue::Gauge(42.0),
                    labels: HashMap::new(),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::from([
                    ("name".to_string(), "test_metric".to_string()),
                    ("source".to_string(), "test".to_string()),
                ]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            },
            TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name: "other_metric".to_string(),
                    description: Some("Other metric".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: MetricType::Gauge,
                    value: MetricValue::Gauge(100.0),
                    labels: HashMap::new(),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::from([
                    ("name".to_string(), "other_metric".to_string()),
                    ("source".to_string(), "test".to_string()),
                ]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            },
        ];

        let batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "test".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::new(),
        };

        // Process the batch
        let processed_batch = processor.process(batch).await.unwrap();

        // Verify that ProcessedRecord conversion worked correctly
        assert!(matches!(processed_batch.status, bridge_core::types::ProcessingStatus::Success));
        assert_eq!(processed_batch.records.len(), 1); // Only one record should pass the filter

        let processed_record = &processed_batch.records[0];
        assert!(matches!(processed_record.status, bridge_core::types::ProcessingStatus::Success));
        assert!(processed_record.transformed_data.is_some());
        assert!(processed_record.errors.is_empty());

        // Verify the filtered record has the correct data
        if let Some(TelemetryData::Metric(metric_data)) = &processed_record.transformed_data {
            assert_eq!(metric_data.name, "test_metric");
            assert_eq!(processed_record.metadata.get("name"), Some(&"test_metric".to_string()));
        } else {
            panic!("Expected metric data");
        }
    }

    #[tokio::test]
    async fn test_filter_processor_empty_result() {
        // Create a filter processor with a rule that won't match any records
        let filter_rules = vec![FilterRule {
            name: "no_match_rule".to_string(),
            field: "name".to_string(),
            operator: FilterOperator::Equals,
            value: "nonexistent".to_string(),
            enabled: true,
        }];

        let config = FilterProcessorConfig::new(filter_rules, FilterMode::Include);
        let processor = FilterProcessor::new(&config).await.unwrap();

        // Create test records
        let records = vec![
            TelemetryRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                record_type: TelemetryType::Metric,
                data: TelemetryData::Metric(MetricData {
                    name: "test_metric".to_string(),
                    description: Some("Test metric".to_string()),
                    unit: Some("count".to_string()),
                    metric_type: MetricType::Gauge,
                    value: MetricValue::Gauge(42.0),
                    labels: HashMap::new(),
                    timestamp: Utc::now(),
                }),
                attributes: HashMap::from([
                    ("name".to_string(), "test_metric".to_string()),
                ]),
                tags: HashMap::new(),
                resource: None,
                service: None,
            },
        ];

        let batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "test".to_string(),
            size: records.len(),
            records,
            metadata: HashMap::new(),
        };

        // Process the batch
        let processed_batch = processor.process(batch).await.unwrap();

        // Verify that no records passed the filter
        assert!(matches!(processed_batch.status, bridge_core::types::ProcessingStatus::Success));
        assert_eq!(processed_batch.records.len(), 0); // No records should pass the filter
        assert!(processed_batch.errors.is_empty());
    }
}
