//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Transform processor implementation
//! 
//! This module provides a transform processor for streaming data transformation.

use async_trait::async_trait;
use bridge_core::{
    BridgeResult, TelemetryBatch,
    types::{TelemetryRecord, TelemetryData, TelemetryType, MetricValue, MetricData, ServiceInfo},
    traits::{StreamProcessor as BridgeStreamProcessor, StreamProcessorStats, DataStream},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use super::ProcessorConfig;

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
    
    /// Transform rule type
    pub rule_type: TransformRuleType,
    
    /// Source field
    pub source_field: String,
    
    /// Target field
    pub target_field: String,
    
    /// Transform value (for set/add operations)
    pub transform_value: Option<String>,
    
    /// Whether rule is enabled
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
        for rule in &self.transform_rules {
            if rule.name.is_empty() {
                return Err(bridge_core::BridgeError::configuration(
                    "Transform rule name cannot be empty".to_string()
                ));
            }
            
            if rule.target_field.is_empty() {
                return Err(bridge_core::BridgeError::configuration(
                    format!("Transform rule '{}' target field cannot be empty", rule.name)
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
    config: TransformProcessorConfig,
    stats: Arc<RwLock<StreamProcessorStats>>,
}

impl TransformProcessor {
    /// Create new transform processor
    pub async fn new(config: &dyn ProcessorConfig) -> BridgeResult<Self> {
        let config = config.as_any()
            .downcast_ref::<TransformProcessorConfig>()
            .ok_or_else(|| bridge_core::BridgeError::configuration(
                "Invalid transform processor configuration".to_string()
            ))?
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
    
    /// Process data stream with transformation logic
    async fn process_data_stream(&self, input: DataStream) -> BridgeResult<DataStream> {
        let start_time = std::time::Instant::now();
        
        info!("Processing data stream with transformations: {}", input.stream_id);
        
        // Deserialize the data stream to TelemetryBatch
        let batch: TelemetryBatch = serde_json::from_slice(&input.data)
            .map_err(|e| bridge_core::BridgeError::internal(
                format!("Failed to deserialize data stream: {}", e)
            ))?;
        
        // Apply transformation rules to each record
        let transformed_records: Vec<TelemetryRecord> = batch.records
            .into_iter()
            .map(|record| self.apply_transformations(record))
            .collect();
        
        // Create transformed batch
        let transformed_batch = TelemetryBatch {
            id: batch.id,
            timestamp: batch.timestamp,
            source: batch.source,
            size: transformed_records.len(),
            records: transformed_records,
            metadata: batch.metadata,
        };
        
        // Serialize transformed batch back to stream
        let transformed_data = serde_json::to_vec(&transformed_batch)
            .map_err(|e| bridge_core::BridgeError::internal(
                format!("Failed to serialize transformed batch: {}", e)
            ))?;
        
        // Update metadata
        let mut output_metadata = input.metadata.clone();
        output_metadata.insert("transformed_by".to_string(), self.config.name.clone());
        output_metadata.insert("transformed_at".to_string(), Utc::now().to_rfc3339());
        output_metadata.insert("transform_rules_applied".to_string(), self.config.transform_rules.len().to_string());
        output_metadata.insert("original_size".to_string(), batch.size.to_string());
        output_metadata.insert("transformed_size".to_string(), transformed_batch.size.to_string());
        
        let processed_stream = DataStream {
            stream_id: format!("transformed_{}", input.stream_id),
            data: transformed_data,
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
            "Data stream transformed: {} records in {}ms",
            batch.size,
            processing_time
        );
        
        Ok(processed_stream)
    }
    
    /// Apply transformation rules to a single record
    fn apply_transformations(&self, mut record: TelemetryRecord) -> TelemetryRecord {
        for rule in &self.config.transform_rules {
            if !rule.enabled {
                continue;
            }
            
            match rule.rule_type {
                TransformRuleType::Copy => {
                    self.copy_field(&mut record, rule);
                }
                TransformRuleType::Rename => {
                    self.rename_field(&mut record, rule);
                }
                TransformRuleType::Set => {
                    self.set_field(&mut record, rule);
                }
                TransformRuleType::Remove => {
                    self.remove_field(&mut record, rule);
                }
                TransformRuleType::Add => {
                    self.add_field(&mut record, rule);
                }
                TransformRuleType::Replace => {
                    self.replace_field(&mut record, rule);
                }
            }
        }
        
        record
    }
    
    /// Copy a field from source to target
    fn copy_field(&self, record: &mut TelemetryRecord, rule: &TransformRule) {
        let source_value = self.get_field_value(record, &rule.source_field);
        if let Some(value) = source_value {
            self.set_field_value(record, &rule.target_field, value);
        }
    }
    
    /// Rename a field (copy from source to target, then remove source)
    fn rename_field(&self, record: &mut TelemetryRecord, rule: &TransformRule) {
        let source_value = self.get_field_value(record, &rule.source_field);
        if let Some(value) = source_value {
            self.set_field_value(record, &rule.target_field, value);
            self.remove_field_value(record, &rule.source_field);
        }
    }
    
    /// Set a field to a specific value
    fn set_field(&self, record: &mut TelemetryRecord, rule: &TransformRule) {
        if let Some(value) = &rule.transform_value {
            self.set_field_value(record, &rule.target_field, value.clone());
        }
    }
    
    /// Remove a field
    fn remove_field(&self, record: &mut TelemetryRecord, rule: &TransformRule) {
        self.remove_field_value(record, &rule.source_field);
    }
    
    /// Add a field (only if it doesn't exist)
    fn add_field(&self, record: &mut TelemetryRecord, rule: &TransformRule) {
        let existing_value = self.get_field_value(record, &rule.target_field);
        if existing_value.is_none() {
            if let Some(value) = &rule.transform_value {
                self.set_field_value(record, &rule.target_field, value.clone());
            }
        }
    }
    
    /// Replace a field value
    fn replace_field(&self, record: &mut TelemetryRecord, rule: &TransformRule) {
        let source_value = self.get_field_value(record, &rule.source_field);
        if source_value.is_some() {
            if let Some(value) = &rule.transform_value {
                self.set_field_value(record, &rule.target_field, value.clone());
            }
        }
    }
    
    /// Get field value from a telemetry record
    fn get_field_value(&self, record: &TelemetryRecord, field_path: &str) -> Option<String> {
        // Simple field path resolution
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
                            "metric.description" => Some(metric_data.description.clone().unwrap_or_default()),
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
    
    /// Set field value in a telemetry record
    fn set_field_value(&self, record: &mut TelemetryRecord, field_path: &str, value: String) {
        match field_path {
            "tags" => {
                // Extract tag name from target_field (e.g., "tags.service" -> "service")
                if let Some(tag_name) = field_path.strip_prefix("tags.") {
                    record.tags.insert(tag_name.to_string(), value);
                }
            }
            "attributes" => {
                // Extract attribute name from target_field (e.g., "attributes.service" -> "service")
                if let Some(attr_name) = field_path.strip_prefix("attributes.") {
                    record.attributes.insert(attr_name.to_string(), value);
                }
            }
            _ => {
                // Add to attributes by default
                record.attributes.insert(field_path.to_string(), value);
            }
        }
    }
    
    /// Remove field value from a telemetry record
    fn remove_field_value(&self, record: &mut TelemetryRecord, field_path: &str) {
        match field_path {
            "tags" => {
                if let Some(tag_name) = field_path.strip_prefix("tags.") {
                    record.tags.remove(tag_name);
                }
            }
            "attributes" => {
                if let Some(attr_name) = field_path.strip_prefix("attributes.") {
                    record.attributes.remove(attr_name);
                }
            }
            _ => {
                record.attributes.remove(field_path);
            }
        }
    }
}

#[async_trait]
impl BridgeStreamProcessor for TransformProcessor {
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
        info!("Shutting down transform processor: {}", self.config.name);
        Ok(())
    }
}
