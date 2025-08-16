//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Transform processor implementation
//!
//! This module provides a transform processor for transforming telemetry data
//! during processing.

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

use super::{BaseProcessor, ProcessorConfig};

/// Transform processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformProcessorConfig {
    /// Processor name
    pub name: String,

    /// Processor version
    pub version: String,

    /// Transform rules
    pub transform_rules: Vec<TransformRule>,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Transform rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformRule {
    /// Rule name
    pub name: String,

    /// Rule type
    pub rule_type: TransformRuleType,

    /// Source field
    pub source_field: String,

    /// Target field
    pub target_field: String,

    /// Transform value
    pub transform_value: Option<String>,

    /// Rule enabled
    pub enabled: bool,
}

/// Transform rule type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformRuleType {
    Copy,
    Rename,
    Set,
    Remove,
    Add,
    Replace,
}

impl TransformProcessorConfig {
    /// Create new transform processor configuration
    pub fn new(transform_rules: Vec<TransformRule>) -> Self {
        Self {
            name: "transform".to_string(),
            version: "1.0.0".to_string(),
            transform_rules,
            additional_config: HashMap::new(),
        }
    }
}

#[async_trait]
impl ProcessorConfig for TransformProcessorConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn validate(&self) -> BridgeResult<()> {
        // Validate transform rules
        for rule in &self.transform_rules {
            if rule.name.is_empty() {
                return Err(bridge_core::BridgeError::configuration(
                    "Transform rule name cannot be empty",
                ));
            }

            if rule.source_field.is_empty() {
                return Err(bridge_core::BridgeError::configuration(
                    "Transform rule source field cannot be empty",
                ));
            }

            if rule.target_field.is_empty() {
                return Err(bridge_core::BridgeError::configuration(
                    "Transform rule target field cannot be empty",
                ));
            }
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Transform processor implementation
pub struct TransformProcessor {
    base: BaseProcessor,
    config: TransformProcessorConfig,
}

impl TransformProcessor {
    /// Create new transform processor
    pub async fn new(config: &dyn ProcessorConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<TransformProcessorConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration("Invalid transform processor configuration")
            })?
            .clone();

        config.validate().await?;

        let base = BaseProcessor::new(config.name.clone(), config.version.clone());

        Ok(Self { base, config })
    }

    /// Apply transform rules to a record
    fn apply_transforms(&self, mut record: TelemetryRecord) -> TelemetryRecord {
        for rule in &self.config.transform_rules {
            if !rule.enabled {
                continue;
            }

            record = self.apply_transform_rule(record, rule);
        }

        record
    }

    /// Apply a single transform rule
    fn apply_transform_rule(
        &self,
        mut record: TelemetryRecord,
        rule: &TransformRule,
    ) -> TelemetryRecord {
        match rule.rule_type {
            TransformRuleType::Copy => {
                let source_value = self.get_field_value(&record, &rule.source_field);
                self.set_field_value(&mut record, &rule.target_field, &source_value);
            }
            TransformRuleType::Rename => {
                let source_value = self.get_field_value(&record, &rule.source_field);
                self.set_field_value(&mut record, &rule.target_field, &source_value);
                self.remove_field_value(&mut record, &rule.source_field);
            }
            TransformRuleType::Set => {
                if let Some(value) = &rule.transform_value {
                    self.set_field_value(&mut record, &rule.target_field, value);
                }
            }
            TransformRuleType::Remove => {
                self.remove_field_value(&mut record, &rule.source_field);
            }
            TransformRuleType::Add => {
                if let Some(value) = &rule.transform_value {
                    self.set_field_value(&mut record, &rule.target_field, value);
                }
            }
            TransformRuleType::Replace => {
                let source_value = self.get_field_value(&record, &rule.source_field);
                if let Some(replace_value) = &rule.transform_value {
                    let new_value = source_value.replace(&rule.source_field, replace_value);
                    self.set_field_value(&mut record, &rule.target_field, &new_value);
                }
            }
        }

        record
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

    /// Set field value in record
    fn set_field_value(&self, record: &mut TelemetryRecord, field: &str, value: &str) {
        // Implement field value setting
        // Set values in record attributes, tags, etc.
        
        // Handle nested field paths (e.g., "attributes.service.name")
        let field_parts: Vec<&str> = field.split('.').collect();
        
        match field_parts.as_slice() {
            ["attributes", key] => {
                record.attributes.insert(key.to_string(), value.to_string());
            }
            ["tags", key] => {
                record.tags.insert(key.to_string(), value.to_string());
            }
            ["data", "metric", "name"] => {
                if let TelemetryData::Metric(ref mut metric) = record.data {
                    metric.name = value.to_string();
                }
            }
            ["data", "log", "message"] => {
                if let TelemetryData::Log(ref mut log) = record.data {
                    log.message = value.to_string();
                }
            }
            ["data", "event", "name"] => {
                if let TelemetryData::Event(ref mut event) = record.data {
                    event.name = value.to_string();
                }
            }
            _ => {
                // Default to setting in attributes
                record.attributes.insert(field.to_string(), value.to_string());
            }
        }
    }

    /// Remove field value from record
    fn remove_field_value(&self, record: &mut TelemetryRecord, field: &str) {
        // Implement field value removal
        // Remove values from record attributes, tags, etc.
        
        // Handle nested field paths (e.g., "attributes.service.name")
        let field_parts: Vec<&str> = field.split('.').collect();
        
        match field_parts.as_slice() {
            ["attributes", key] => {
                record.attributes.remove(*key);
            }
            ["tags", key] => {
                record.tags.remove(*key);
            }
            _ => {
                // Try to remove from attributes first, then tags
                record.attributes.remove(field);
                record.tags.remove(field);
            }
        }
    }
}

#[async_trait]
impl BridgeTelemetryProcessor for TransformProcessor {
    async fn process(&self, batch: TelemetryBatch) -> BridgeResult<ProcessedBatch> {
        let start_time = std::time::Instant::now();

        // Transform records based on rules
        let transformed_records: Vec<TelemetryRecord> = batch
            .records
            .into_iter()
            .map(|record| self.apply_transforms(record))
            .collect();

        let transformed_batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: batch.source,
            size: transformed_records.len(),
            records: transformed_records,
            metadata: batch.metadata,
        };

        let processing_time = start_time.elapsed();
        let processing_time_ms = processing_time.as_millis() as f64;

        // Update statistics
        self.base
            .update_stats(1, transformed_batch.size, processing_time_ms)
            .await;

        Ok(ProcessedBatch {
            original_batch_id: batch.id,
            timestamp: Utc::now(),
            status: bridge_core::types::ProcessingStatus::Success,
            records: transformed_batch
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
            metadata: transformed_batch.metadata,
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
        info!("Transform processor shutdown completed");
        Ok(())
    }
}

impl TransformProcessor {
    /// Get transform configuration
    pub fn get_config(&self) -> &TransformProcessorConfig {
        &self.config
    }

    /// Add transform rule
    pub fn add_transform_rule(&mut self, rule: TransformRule) {
        self.config.transform_rules.push(rule);
    }

    /// Remove transform rule by name
    pub fn remove_transform_rule(&mut self, rule_name: &str) {
        self.config
            .transform_rules
            .retain(|rule| rule.name != rule_name);
    }
}
