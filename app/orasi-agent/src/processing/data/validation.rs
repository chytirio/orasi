//! Data validation processing

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

/// Validation processor for handling data validation tasks
pub struct ValidationProcessor {
    config: AgentConfig,
    state: Arc<RwLock<AgentState>>,
}

impl ValidationProcessor {
    /// Create new validation processor
    pub fn new(config: &AgentConfig, state: Arc<RwLock<AgentState>>) -> Self {
        Self {
            config: config.clone(),
            state,
        }
    }

    /// Execute validation pipeline
    pub async fn execute_pipeline(&self, task: &ProcessingTask) -> Result<Value, AgentError> {
        info!("Executing validation pipeline: {}", task.pipeline);

        // Simulate reading input data
        let input_records = DataIO::simulate_read_input_data(&task.input_location).await?;
        let validation_rules = self.parse_validation_rules(&task.options)?;
        
        // Apply validation
        let validation_results = self.apply_validation(&input_records, &validation_rules).await?;
        
        // Filter valid records
        let valid_records: Vec<Value> = input_records.clone().into_iter()
            .zip(validation_results.iter())
            .filter_map(|(record, result)| {
                if result.get("is_valid").and_then(|v| v.as_bool()).unwrap_or(false) {
                    Some(record)
                } else {
                    None
                }
            })
            .collect();
        
        // Simulate writing output data
        let output_records = valid_records.len();
        DataIO::simulate_write_output_data(&task.output_destination, &valid_records).await?;

        let result = serde_json::json!({
            "pipeline_type": "validation",
            "pipeline": task.pipeline,
            "input_records": input_records.len(),
            "valid_records": output_records,
            "invalid_records": input_records.len() - output_records,
            "validation_rules": validation_rules,
            "validation_results": validation_results,
            "status": "completed"
        });

        Ok(result)
    }
    
    /// Parse validation rules from options
    fn parse_validation_rules(&self, options: &HashMap<String, String>) -> Result<Value, AgentError> {
        let rules = ConfigParser::parse_array_config(options, "rules")?;
        Ok(serde_json::Value::Array(rules))
    }
    
    /// Apply validation to input records
    async fn apply_validation(&self, records: &[Value], rules: &Value) -> Result<Vec<Value>, AgentError> {
        let empty_vec = Vec::new();
        let validation_rules = rules.as_array().unwrap_or(&empty_vec);
        let mut validation_results = Vec::new();
        
        for record in records {
            let mut record_validation = serde_json::Map::new();
            let mut is_valid = true;
            let mut validation_errors = Vec::new();
            
            for rule in validation_rules {
                let rule_result = self.apply_validation_rule(record, rule).await?;
                
                if let Some(rule_name) = rule.get("name").and_then(|v| v.as_str()) {
                    record_validation.insert(rule_name.to_string(), rule_result.clone());
                    
                    if !rule_result.get("is_valid").and_then(|v| v.as_bool()).unwrap_or(true) {
                        is_valid = false;
                        if let Some(error_msg) = rule_result.get("error").and_then(|v| v.as_str()) {
                            validation_errors.push(error_msg.to_string());
                        }
                    }
                }
            }
            
            record_validation.insert("is_valid".to_string(), serde_json::Value::Bool(is_valid));
            record_validation.insert("errors".to_string(), serde_json::Value::Array(
                validation_errors.into_iter().map(|e| serde_json::Value::String(e)).collect()
            ));
            
            validation_results.push(serde_json::Value::Object(record_validation));
        }
        
        Ok(validation_results)
    }
    
    /// Apply a single validation rule
    async fn apply_validation_rule(&self, record: &Value, rule: &Value) -> Result<Value, AgentError> {
        let rule_type = rule.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let field = rule.get("field").and_then(|v| v.as_str()).unwrap_or("");
        let parameters = rule.get("parameters").unwrap_or(&serde_json::Value::Null);
        
        let field_value = record.get(field);
        
        match rule_type {
            "required" => {
                let is_valid = field_value.is_some() && !field_value.unwrap().is_null();
                Ok(serde_json::json!({
                    "is_valid": is_valid,
                    "error": if is_valid { serde_json::Value::Null } else { serde_json::Value::String("Field is required".to_string()) }
                }))
            }
            "not_null" => {
                let is_valid = field_value.is_some() && !field_value.unwrap().is_null();
                Ok(serde_json::json!({
                    "is_valid": is_valid,
                    "error": if is_valid { serde_json::Value::Null } else { serde_json::Value::String("Field cannot be null".to_string()) }
                }))
            }
            "string_length" => {
                if let Some(min_length) = parameters.get("min_length").and_then(|v| v.as_u64()) {
                    if let Some(max_length) = parameters.get("max_length").and_then(|v| v.as_u64()) {
                        if let Some(value) = field_value.and_then(|v| v.as_str()) {
                            let length = value.len() as u64;
                            let is_valid = length >= min_length && length <= max_length;
                            Ok(serde_json::json!({
                                "is_valid": is_valid,
                                "error": if is_valid { serde_json::Value::Null } else { serde_json::Value::String(format!("String length must be between {} and {}", min_length, max_length)) }
                            }))
                        } else {
                            Ok(serde_json::json!({
                                "is_valid": false,
                                "error": "Field is not a string"
                            }))
                        }
                    } else {
                        Ok(serde_json::json!({
                            "is_valid": false,
                            "error": "Invalid max_length parameter"
                        }))
                    }
                } else {
                    Ok(serde_json::json!({
                        "is_valid": false,
                        "error": "Invalid min_length parameter"
                    }))
                }
            }
            "numeric_range" => {
                if let Some(min_value) = parameters.get("min_value").and_then(|v| v.as_f64()) {
                    if let Some(max_value) = parameters.get("max_value").and_then(|v| v.as_f64()) {
                        if let Some(value) = field_value.and_then(|v| v.as_f64()) {
                            let is_valid = value >= min_value && value <= max_value;
                            Ok(serde_json::json!({
                                "is_valid": is_valid,
                                "error": if is_valid { serde_json::Value::Null } else { serde_json::Value::String(format!("Value must be between {} and {}", min_value, max_value)) }
                            }))
                        } else {
                            Ok(serde_json::json!({
                                "is_valid": false,
                                "error": "Field is not a number"
                            }))
                        }
                    } else {
                        Ok(serde_json::json!({
                            "is_valid": false,
                            "error": "Invalid max_value parameter"
                        }))
                    }
                } else {
                    Ok(serde_json::json!({
                        "is_valid": false,
                        "error": "Invalid min_value parameter"
                    }))
                }
            }
            "regex_pattern" => {
                if let Some(pattern) = parameters.get("pattern").and_then(|v| v.as_str()) {
                    if let Some(value) = field_value.and_then(|v| v.as_str()) {
                        // Simple regex validation - in a real implementation, use a proper regex library
                        let is_valid = self.simple_regex_match(value, pattern);
                        Ok(serde_json::json!({
                            "is_valid": is_valid,
                            "error": if is_valid { serde_json::Value::Null } else { serde_json::Value::String(format!("Value does not match pattern: {}", pattern)) }
                        }))
                    } else {
                        Ok(serde_json::json!({
                            "is_valid": false,
                            "error": "Field is not a string"
                        }))
                    }
                } else {
                    Ok(serde_json::json!({
                        "is_valid": false,
                        "error": "Invalid pattern parameter"
                    }))
                }
            }
            "enum_values" => {
                if let Some(allowed_values) = parameters.get("values").and_then(|v| v.as_array()) {
                    if let Some(value) = field_value {
                        let is_valid = allowed_values.contains(value);
                        Ok(serde_json::json!({
                            "is_valid": is_valid,
                            "error": if is_valid { serde_json::Value::Null } else { serde_json::Value::String(format!("Value must be one of: {:?}", allowed_values)) }
                        }))
                    } else {
                        Ok(serde_json::json!({
                            "is_valid": false,
                            "error": "Field is required"
                        }))
                    }
                } else {
                    Ok(serde_json::json!({
                        "is_valid": false,
                        "error": "Invalid values parameter"
                    }))
                }
            }
            "custom" => {
                if let Some(expression) = parameters.get("expression").and_then(|v| v.as_str()) {
                    let is_valid = self.evaluate_custom_validation(record, expression)?;
                    Ok(serde_json::json!({
                        "is_valid": is_valid,
                        "error": if is_valid { serde_json::Value::Null } else { serde_json::Value::String("Custom validation failed".to_string()) }
                    }))
                } else {
                    Ok(serde_json::json!({
                        "is_valid": false,
                        "error": "Invalid expression parameter"
                    }))
                }
            }
            _ => {
                Ok(serde_json::json!({
                    "is_valid": false,
                    "error": format!("Unknown validation type: {}", rule_type)
                }))
            }
        }
    }
    
    /// Simple regex matching (placeholder implementation)
    fn simple_regex_match(&self, value: &str, pattern: &str) -> bool {
        // This is a simplified regex matcher - in a real implementation, use a proper regex library
        match pattern {
            "^[a-zA-Z]+$" => value.chars().all(|c| c.is_alphabetic()),
            "^[0-9]+$" => value.chars().all(|c| c.is_numeric()),
            "^[a-zA-Z0-9]+$" => value.chars().all(|c| c.is_alphanumeric()),
            "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$" => {
                // Simple email validation
                value.contains('@') && value.contains('.')
            }
            _ => {
                // Default to true for unknown patterns
                true
            }
        }
    }
    
    /// Evaluate custom validation expression
    fn evaluate_custom_validation(&self, record: &Value, expression: &str) -> Result<bool, AgentError> {
        // Simple custom validation - in a real implementation, use a proper expression engine
        match expression {
            "has_required_fields" => {
                let required_fields = vec!["id", "name", "timestamp"];
                Ok(required_fields.iter().all(|field| record.get(*field).is_some()))
            }
            "valid_timestamp" => {
                if let Some(timestamp) = record.get("timestamp").and_then(|v| v.as_u64()) {
                    Ok(timestamp > 0)
                } else {
                    Ok(false)
                }
            }
            "positive_value" => {
                if let Some(value) = record.get("value").and_then(|v| v.as_f64()) {
                    Ok(value > 0.0)
                } else {
                    Ok(false)
                }
            }
            _ => {
                Ok(true) // Default to valid for unknown expressions
            }
        }
    }
}

impl BaseProcessor for ValidationProcessor {
    fn config(&self) -> &AgentConfig {
        &self.config
    }
    
    fn state(&self) -> &Arc<RwLock<AgentState>> {
        &self.state
    }
}
