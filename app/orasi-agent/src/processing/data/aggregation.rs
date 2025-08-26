//! Data aggregation processing

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::ProcessingTask;
use crate::state::AgentState;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::utils::{BaseProcessor, ConfigParser, DataIO};

/// Aggregation processor for handling data aggregation tasks
pub struct AggregationProcessor {
    config: AgentConfig,
    state: Arc<RwLock<AgentState>>,
}

impl AggregationProcessor {
    /// Create new aggregation processor
    pub fn new(config: &AgentConfig, state: Arc<RwLock<AgentState>>) -> Self {
        Self {
            config: config.clone(),
            state,
        }
    }

    /// Execute aggregation pipeline
    pub async fn execute_pipeline(&self, task: &ProcessingTask) -> Result<Value, AgentError> {
        info!("Executing aggregation pipeline: {}", task.pipeline);

        // Simulate reading input data
        let input_records = DataIO::simulate_read_input_data(&task.input_location).await?;
        let aggregation_config = self.parse_aggregation_config(&task.options)?;
        
        // Apply aggregations
        let aggregated_results = self.apply_aggregations(&input_records, &aggregation_config).await?;
        
        // Simulate writing output data
        let output_records = aggregated_results.len();
        DataIO::simulate_write_output_data(&task.output_destination, &aggregated_results).await?;

        let result = serde_json::json!({
            "pipeline_type": "aggregation",
            "pipeline": task.pipeline,
            "input_records": input_records.len(),
            "aggregated_groups": output_records,
            "aggregation_functions": aggregation_config.get("functions").unwrap_or(&serde_json::Value::Null),
            "group_by_fields": aggregation_config.get("group_by").unwrap_or(&serde_json::Value::Null),
            "aggregated_results": aggregated_results,
            "status": "completed"
        });

        Ok(result)
    }
    
    /// Parse aggregation configuration from options
    fn parse_aggregation_config(&self, options: &HashMap<String, String>) -> Result<Value, AgentError> {
        ConfigParser::parse_json_config(options, "functions")
    }
    
    /// Apply aggregations to input records
    async fn apply_aggregations(&self, records: &[Value], config: &Value) -> Result<Vec<Value>, AgentError> {
        let empty_vec = Vec::new();
        let group_by_fields = config.get("group_by")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);
        
        let aggregation_functions = config.get("functions")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);
        
        // Group records by specified fields
        let mut grouped_data: HashMap<String, Vec<&Value>> = HashMap::new();
        
        for record in records {
            let group_key = self.generate_group_key(record, group_by_fields)?;
            grouped_data.entry(group_key).or_insert_with(Vec::new).push(record);
        }
        
        // Apply aggregation functions to each group
        let mut aggregated_results = Vec::new();
        
        for (group_key, group_records) in grouped_data {
            let mut aggregated_record = serde_json::Map::new();
            
            // Add group key fields
            let group_parts: Vec<&str> = group_key.split('|').collect();
            for (i, field) in group_by_fields.iter().enumerate() {
                if let Some(field_name) = field.as_str() {
                    aggregated_record.insert(field_name.to_string(), serde_json::Value::String(group_parts.get(i).unwrap_or(&"").to_string()));
                }
            }
            
            // Apply aggregation functions
            for function_config in aggregation_functions {
                let function_name = function_config.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let field_name = function_config.get("field").and_then(|v| v.as_str()).unwrap_or("");
                let default_result_field = format!("{}_{}", function_name, field_name);
                let result_field = function_config.get("result_field").and_then(|v| v.as_str()).unwrap_or(&default_result_field);
                
                let aggregated_value = self.apply_aggregation_function(&group_records, function_name, field_name).await?;
                aggregated_record.insert(result_field.to_string(), aggregated_value);
            }
            
            aggregated_results.push(serde_json::Value::Object(aggregated_record));
        }
        
        Ok(aggregated_results)
    }
    
    /// Generate group key for aggregation
    fn generate_group_key(&self, record: &Value, group_by_fields: &[Value]) -> Result<String, AgentError> {
        let mut key_parts = Vec::new();
        
        for field in group_by_fields {
            if let Some(field_name) = field.as_str() {
                let field_value = record.get(field_name)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "null".to_string());
                key_parts.push(field_value);
            }
        }
        
        Ok(key_parts.join("|"))
    }
    
    /// Apply a single aggregation function
    async fn apply_aggregation_function(
        &self, 
        records: &[&Value], 
        function_name: &str, 
        field_name: &str
    ) -> Result<Value, AgentError> {
        let values: Vec<f64> = records.iter()
            .filter_map(|record| record.get(field_name))
            .filter_map(|value| value.as_f64())
            .collect();
        
        if values.is_empty() {
            return Ok(serde_json::Value::Null);
        }
        
        let result = match function_name {
            "count" => {
                serde_json::Value::Number(serde_json::Number::from(values.len() as u64))
            }
            "sum" => {
                let sum: f64 = values.iter().sum();
                serde_json::Value::Number(serde_json::Number::from_f64(sum).unwrap_or(serde_json::Number::from(0)))
            }
            "avg" | "average" => {
                let sum: f64 = values.iter().sum();
                let avg = sum / values.len() as f64;
                serde_json::Value::Number(serde_json::Number::from_f64(avg).unwrap_or(serde_json::Number::from(0)))
            }
            "min" => {
                let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                serde_json::Value::Number(serde_json::Number::from_f64(min).unwrap_or(serde_json::Number::from(0)))
            }
            "max" => {
                let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                serde_json::Value::Number(serde_json::Number::from_f64(max).unwrap_or(serde_json::Number::from(0)))
            }
            "median" => {
                let mut sorted_values = values.clone();
                sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let mid = sorted_values.len() / 2;
                let median = if sorted_values.len() % 2 == 0 {
                    (sorted_values[mid - 1] + sorted_values[mid]) / 2.0
                } else {
                    sorted_values[mid]
                };
                serde_json::Value::Number(serde_json::Number::from_f64(median).unwrap_or(serde_json::Number::from(0)))
            }
            "stddev" | "standard_deviation" => {
                let avg: f64 = values.iter().sum::<f64>() / values.len() as f64;
                let variance: f64 = values.iter()
                    .map(|x| (x - avg).powi(2))
                    .sum::<f64>() / values.len() as f64;
                let stddev = variance.sqrt();
                serde_json::Value::Number(serde_json::Number::from_f64(stddev).unwrap_or(serde_json::Number::from(0)))
            }
            "variance" => {
                let avg: f64 = values.iter().sum::<f64>() / values.len() as f64;
                let variance: f64 = values.iter()
                    .map(|x| (x - avg).powi(2))
                    .sum::<f64>() / values.len() as f64;
                serde_json::Value::Number(serde_json::Number::from_f64(variance).unwrap_or(serde_json::Number::from(0)))
            }
            _ => {
                return Err(AgentError::InvalidInput(format!("Unknown aggregation function: {}", function_name)));
            }
        };
        
        Ok(result)
    }
}

impl BaseProcessor for AggregationProcessor {
    fn config(&self) -> &AgentConfig {
        &self.config
    }
    
    fn state(&self) -> &Arc<RwLock<AgentState>> {
        &self.state
    }
}
