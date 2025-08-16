//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Aggregate processor implementation
//!
//! This module provides an aggregate processor for streaming data aggregation.

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
use tracing::info;

use super::ProcessorConfig;

/// Aggregate processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateProcessorConfig {
    /// Processor name
    pub name: String,

    /// Processor version
    pub version: String,

    /// Aggregation rules
    pub aggregation_rules: Vec<AggregationRule>,

    /// Window size in milliseconds
    pub window_size_ms: u64,

    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Aggregation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationRule {
    /// Rule name
    pub name: String,

    /// Group by fields
    pub group_by_fields: Vec<String>,

    /// Aggregation functions
    pub aggregation_functions: Vec<AggregationFunction>,

    /// Whether rule is enabled
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
}

/// Aggregation function type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AggregationFunctionType {
    Count,
    Sum,
    Average,
    Min,
    Max,
    First,
    Last,
}

impl AggregateProcessorConfig {
    /// Create new aggregate processor configuration
    pub fn new(aggregation_rules: Vec<AggregationRule>, window_size_ms: u64) -> Self {
        Self {
            name: "aggregate".to_string(),
            version: "1.0.0".to_string(),
            aggregation_rules,
            window_size_ms,
            flush_interval_ms: 5000,
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
        for rule in &self.aggregation_rules {
            if rule.name.is_empty() {
                return Err(bridge_core::BridgeError::configuration(
                    "Aggregation rule name cannot be empty".to_string(),
                ));
            }

            if rule.aggregation_functions.is_empty() {
                return Err(bridge_core::BridgeError::configuration(format!(
                    "Aggregation rule '{}' must have at least one function",
                    rule.name
                )));
            }
        }

        if self.window_size_ms == 0 {
            return Err(bridge_core::BridgeError::configuration(
                "Window size cannot be 0".to_string(),
            ));
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Aggregate processor implementation
pub struct AggregateProcessor {
    config: AggregateProcessorConfig,
    stats: Arc<RwLock<StreamProcessorStats>>,
}

impl AggregateProcessor {
    /// Create new aggregate processor
    pub async fn new(config: &dyn ProcessorConfig) -> BridgeResult<Self> {
        let config = config
            .as_any()
            .downcast_ref::<AggregateProcessorConfig>()
            .ok_or_else(|| {
                bridge_core::BridgeError::configuration(
                    "Invalid aggregate processor configuration".to_string(),
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

    /// Process data stream with aggregation logic
    async fn process_data_stream(&self, input: DataStream) -> BridgeResult<DataStream> {
        let start_time = std::time::Instant::now();

        info!(
            "Processing data stream with aggregations: {}",
            input.stream_id
        );

        // Step 1: Deserialize data from the stream
        let records = self.deserialize_data(&input.data).await?;

        // Step 2: Apply aggregation rules
        let aggregated_data = self.apply_aggregation_rules(&records).await?;

        // Step 3: Serialize aggregated data back to stream
        let serialized_data = self.serialize_aggregated_data(&aggregated_data).await?;

        let mut output_metadata = input.metadata.clone();
        output_metadata.insert("aggregated_by".to_string(), self.config.name.clone());
        output_metadata.insert("aggregated_at".to_string(), Utc::now().to_rfc3339());
        output_metadata.insert(
            "aggregation_rules_applied".to_string(),
            self.config.aggregation_rules.len().to_string(),
        );
        output_metadata.insert("input_records_count".to_string(), records.len().to_string());
        output_metadata.insert(
            "output_records_count".to_string(),
            aggregated_data.len().to_string(),
        );

        let processed_stream = DataStream {
            stream_id: input.stream_id,
            data: serialized_data,
            metadata: output_metadata,
            timestamp: Utc::now(),
        };

        let processing_time = start_time.elapsed().as_millis() as f64;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_records += records.len() as u64;
            stats.avg_processing_time_ms = (stats.avg_processing_time_ms + processing_time) / 2.0;
            stats.last_process_time = Some(Utc::now());
        }

        info!(
            "Data stream aggregated in {}ms ({} -> {} records)",
            processing_time,
            records.len(),
            aggregated_data.len()
        );

        Ok(processed_stream)
    }

    /// Deserialize data from the stream
    async fn deserialize_data(&self, data: &[u8]) -> BridgeResult<Vec<serde_json::Value>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        // Try to parse as JSON array
        if let Ok(json_array) = serde_json::from_slice::<Vec<serde_json::Value>>(data) {
            return Ok(json_array);
        }

        // Try to parse as single JSON object
        if let Ok(json_obj) = serde_json::from_slice::<serde_json::Value>(data) {
            return Ok(vec![json_obj]);
        }

        // Try to parse as newline-delimited JSON
        let json_lines: Vec<serde_json::Value> = String::from_utf8_lossy(data)
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    serde_json::from_str(trimmed).ok()
                } else {
                    None
                }
            })
            .collect();

        if !json_lines.is_empty() {
            return Ok(json_lines);
        }

        // If all else fails, create a simple record with the raw data
        let fallback_record = serde_json::json!({
            "raw_data": String::from_utf8_lossy(data),
            "timestamp": Utc::now().to_rfc3339(),
            "data_size": data.len()
        });

        Ok(vec![fallback_record])
    }

    /// Apply aggregation rules to the data
    async fn apply_aggregation_rules(
        &self,
        records: &[serde_json::Value],
    ) -> BridgeResult<Vec<serde_json::Value>> {
        let mut aggregated_records = Vec::new();

        for rule in &self.config.aggregation_rules {
            if !rule.enabled {
                continue;
            }

            let aggregated = self.apply_single_rule(rule, records).await?;
            aggregated_records.extend(aggregated);
        }

        // If no rules were applied, return the original records
        if aggregated_records.is_empty() {
            return Ok(records.to_vec());
        }

        Ok(aggregated_records)
    }

    /// Apply a single aggregation rule
    async fn apply_single_rule(
        &self,
        rule: &AggregationRule,
        records: &[serde_json::Value],
    ) -> BridgeResult<Vec<serde_json::Value>> {
        let mut groups: HashMap<String, Vec<&serde_json::Value>> = HashMap::new();

        // Group records by the specified fields
        for record in records {
            let group_key = self.create_group_key(record, &rule.group_by_fields);
            groups
                .entry(group_key)
                .or_insert_with(Vec::new)
                .push(record);
        }

        let mut aggregated_records = Vec::new();

        // Apply aggregation functions to each group
        for (group_key, group_records) in groups {
            let mut aggregated_record = serde_json::Map::new();

            // Add group key fields to the aggregated record
            if let Some(record) = group_records.first() {
                for field in &rule.group_by_fields {
                    if let Some(value) = record.get(field) {
                        aggregated_record.insert(field.clone(), value.clone());
                    }
                }
            }

            // Apply aggregation functions
            for function in &rule.aggregation_functions {
                let aggregated_value = self
                    .apply_aggregation_function(function, &group_records)
                    .await?;
                aggregated_record.insert(function.target_field.clone(), aggregated_value);
            }

            // Add metadata
            aggregated_record.insert(
                "_group_key".to_string(),
                serde_json::Value::String(group_key),
            );
            aggregated_record.insert(
                "_record_count".to_string(),
                serde_json::Value::Number(serde_json::Number::from(group_records.len())),
            );
            aggregated_record.insert(
                "_aggregated_at".to_string(),
                serde_json::Value::String(Utc::now().to_rfc3339()),
            );
            aggregated_record.insert(
                "_rule_name".to_string(),
                serde_json::Value::String(rule.name.clone()),
            );

            aggregated_records.push(serde_json::Value::Object(aggregated_record));
        }

        Ok(aggregated_records)
    }

    /// Create a group key from record fields
    fn create_group_key(&self, record: &serde_json::Value, group_by_fields: &[String]) -> String {
        let mut key_parts = Vec::new();

        for field in group_by_fields {
            if let Some(value) = record.get(field) {
                key_parts.push(format!("{}:{}", field, value));
            } else {
                key_parts.push(format!("{}:null", field));
            }
        }

        if key_parts.is_empty() {
            "default_group".to_string()
        } else {
            key_parts.join("|")
        }
    }

    /// Apply a single aggregation function to a group of records
    async fn apply_aggregation_function(
        &self,
        function: &AggregationFunction,
        records: &[&serde_json::Value],
    ) -> BridgeResult<serde_json::Value> {
        let values: Vec<f64> = records
            .iter()
            .filter_map(|record| {
                record.get(&function.source_field).and_then(|v| {
                    v.as_f64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                })
            })
            .collect();

        if values.is_empty() {
            return Ok(serde_json::Value::Null);
        }

        let result = match function.function_type {
            AggregationFunctionType::Count => {
                serde_json::Value::Number(serde_json::Number::from(records.len()))
            }
            AggregationFunctionType::Sum => {
                let sum: f64 = values.iter().sum();
                serde_json::Value::Number(
                    serde_json::Number::from_f64(sum).unwrap_or(serde_json::Number::from(0)),
                )
            }
            AggregationFunctionType::Average => {
                let avg = values.iter().sum::<f64>() / values.len() as f64;
                serde_json::Value::Number(
                    serde_json::Number::from_f64(avg).unwrap_or(serde_json::Number::from(0)),
                )
            }
            AggregationFunctionType::Min => {
                let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                serde_json::Value::Number(
                    serde_json::Number::from_f64(min).unwrap_or(serde_json::Number::from(0)),
                )
            }
            AggregationFunctionType::Max => {
                let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                serde_json::Value::Number(
                    serde_json::Number::from_f64(max).unwrap_or(serde_json::Number::from(0)),
                )
            }
            AggregationFunctionType::First => {
                if let Some(first_record) = records.first() {
                    first_record
                        .get(&function.source_field)
                        .cloned()
                        .unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                }
            }
            AggregationFunctionType::Last => {
                if let Some(last_record) = records.last() {
                    last_record
                        .get(&function.source_field)
                        .cloned()
                        .unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                }
            }
        };

        Ok(result)
    }

    /// Serialize aggregated data back to stream format
    async fn serialize_aggregated_data(
        &self,
        aggregated_data: &[serde_json::Value],
    ) -> BridgeResult<Vec<u8>> {
        if aggregated_data.len() == 1 {
            // Single record - serialize as JSON object
            serde_json::to_vec(&aggregated_data[0]).map_err(|e| {
                bridge_core::BridgeError::serialization(format!(
                    "Failed to serialize aggregated data: {}",
                    e
                ))
            })
        } else {
            // Multiple records - serialize as JSON array
            serde_json::to_vec(aggregated_data).map_err(|e| {
                bridge_core::BridgeError::serialization(format!(
                    "Failed to serialize aggregated data: {}",
                    e
                ))
            })
        }
    }
}

#[async_trait]
impl BridgeStreamProcessor for AggregateProcessor {
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
        info!("Shutting down aggregate processor: {}", self.config.name);
        Ok(())
    }
}
