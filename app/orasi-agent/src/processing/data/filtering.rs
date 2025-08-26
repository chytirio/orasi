//! Data filtering processing

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::ProcessingTask;
use crate::state::AgentState;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::utils::{BaseProcessor, ConfigParser, DataIO};

/// Filtering processor for handling data filtering tasks
pub struct FilteringProcessor {
    config: AgentConfig,
    state: Arc<RwLock<AgentState>>,
}

impl FilteringProcessor {
    /// Create new filtering processor
    pub fn new(config: &AgentConfig, state: Arc<RwLock<AgentState>>) -> Self {
        Self {
            config: config.clone(),
            state,
        }
    }

    /// Execute filtering pipeline
    pub async fn execute_pipeline(&self, task: &ProcessingTask) -> Result<Value, AgentError> {
        info!("Executing filtering pipeline: {}", task.pipeline);

        // Simulate reading input data
        let input_records = DataIO::simulate_read_input_data(&task.input_location).await?;
        let filter_criteria = self.parse_filter_criteria(&task.options)?;
        
        // Apply filters
        let filtered_records = self.apply_filters(&input_records, &filter_criteria).await?;
        
        // Simulate writing output data
        let output_records = filtered_records.len();
        DataIO::simulate_write_output_data(&task.output_destination, &filtered_records).await?;

        let result = serde_json::json!({
            "pipeline_type": "filtering",
            "pipeline": task.pipeline,
            "input_records": input_records.len(),
            "filtered_records": output_records,
            "filtered_out_records": input_records.len() - output_records,
            "filter_criteria": filter_criteria,
            "status": "completed"
        });

        Ok(result)
    }
    
    /// Parse filter criteria from options
    fn parse_filter_criteria(&self, options: &HashMap<String, String>) -> Result<Value, AgentError> {
        let criteria = ConfigParser::parse_array_config(options, "criteria")?;
        Ok(serde_json::Value::Array(criteria))
    }
    
    /// Apply filters to input records
    async fn apply_filters(&self, records: &[Value], criteria: &Value) -> Result<Vec<Value>, AgentError> {
        let empty_vec = Vec::new();
        let filter_rules = criteria.as_array().unwrap_or(&empty_vec);
        
        let mut filtered_records = records.to_vec();
        
        for rule in filter_rules {
            filtered_records = self.apply_filter_rule(&filtered_records, rule).await?;
        }
        
        Ok(filtered_records)
    }
    
    /// Apply a single filter rule
    async fn apply_filter_rule(&self, records: &[Value], rule: &Value) -> Result<Vec<Value>, AgentError> {
        let field = rule.get("field").and_then(|v| v.as_str()).unwrap_or("");
        let operator = rule.get("operator").and_then(|v| v.as_str()).unwrap_or("");
        let value = rule.get("value").unwrap_or(&serde_json::Value::Null);
        let action = rule.get("action").and_then(|v| v.as_str()).unwrap_or("include");
        
        let mut filtered_records = Vec::new();
        
        for record in records {
            let should_include = self.evaluate_filter_condition(record, field, operator, value)?;
            
            match action {
                "include" => {
                    if should_include {
                        filtered_records.push(record.clone());
                    }
                }
                "exclude" => {
                    if !should_include {
                        filtered_records.push(record.clone());
                    }
                }
                _ => {
                    return Err(AgentError::InvalidInput(format!("Unknown filter action: {}", action)));
                }
            }
        }
        
        Ok(filtered_records)
    }
    
    /// Evaluate a filter condition
    fn evaluate_filter_condition(
        &self, 
        record: &Value, 
        field: &str, 
        operator: &str, 
        value: &Value
    ) -> Result<bool, AgentError> {
        let field_value = record.get(field);
        
        if field_value.is_none() {
            return Ok(false);
        }
        
        let field_value = field_value.unwrap();
        
        match operator {
            "equals" => {
                Ok(field_value == value)
            }
            "not_equals" => {
                Ok(field_value != value)
            }
            "greater_than" => {
                if let (Some(fv), Some(v)) = (field_value.as_f64(), value.as_f64()) {
                    Ok(fv > v)
                } else {
                    Ok(false)
                }
            }
            "less_than" => {
                if let (Some(fv), Some(v)) = (field_value.as_f64(), value.as_f64()) {
                    Ok(fv < v)
                } else {
                    Ok(false)
                }
            }
            "greater_than_or_equal" => {
                if let (Some(fv), Some(v)) = (field_value.as_f64(), value.as_f64()) {
                    Ok(fv >= v)
                } else {
                    Ok(false)
                }
            }
            "less_than_or_equal" => {
                if let (Some(fv), Some(v)) = (field_value.as_f64(), value.as_f64()) {
                    Ok(fv <= v)
                } else {
                    Ok(false)
                }
            }
            "contains" => {
                if let (Some(fv), Some(v)) = (field_value.as_str(), value.as_str()) {
                    Ok(fv.contains(v))
                } else {
                    Ok(false)
                }
            }
            "not_contains" => {
                if let (Some(fv), Some(v)) = (field_value.as_str(), value.as_str()) {
                    Ok(!fv.contains(v))
                } else {
                    Ok(false)
                }
            }
            "starts_with" => {
                if let (Some(fv), Some(v)) = (field_value.as_str(), value.as_str()) {
                    Ok(fv.starts_with(v))
                } else {
                    Ok(false)
                }
            }
            "ends_with" => {
                if let (Some(fv), Some(v)) = (field_value.as_str(), value.as_str()) {
                    Ok(fv.ends_with(v))
                } else {
                    Ok(false)
                }
            }
            "is_null" => {
                Ok(field_value.is_null())
            }
            "is_not_null" => {
                Ok(!field_value.is_null())
            }
            "in" => {
                if let Some(value_array) = value.as_array() {
                    Ok(value_array.contains(field_value))
                } else {
                    Ok(false)
                }
            }
            "not_in" => {
                if let Some(value_array) = value.as_array() {
                    Ok(!value_array.contains(field_value))
                } else {
                    Ok(false)
                }
            }
            "between" => {
                if let Some(value_array) = value.as_array() {
                    if value_array.len() == 2 {
                        if let (Some(fv), Some(min), Some(max)) = (
                            field_value.as_f64(),
                            value_array[0].as_f64(),
                            value_array[1].as_f64()
                        ) {
                            Ok(fv >= min && fv <= max)
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
            _ => {
                return Err(AgentError::InvalidInput(format!("Unknown filter operator: {}", operator)));
            }
        }
    }
}

impl BaseProcessor for FilteringProcessor {
    fn config(&self) -> &AgentConfig {
        &self.config
    }
    
    fn state(&self) -> &Arc<RwLock<AgentState>> {
        &self.state
    }
}
