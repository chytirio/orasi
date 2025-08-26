//! Data transformation processing

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

/// Transformation processor for handling data transformation tasks
pub struct TransformationProcessor {
    config: AgentConfig,
    state: Arc<RwLock<AgentState>>,
}

impl TransformationProcessor {
    /// Create new transformation processor
    pub fn new(config: &AgentConfig, state: Arc<RwLock<AgentState>>) -> Self {
        Self {
            config: config.clone(),
            state,
        }
    }

    /// Execute transformation pipeline
    pub async fn execute_pipeline(&self, task: &ProcessingTask) -> Result<Value, AgentError> {
        info!("Executing transformation pipeline: {}", task.pipeline);

        // Simulate reading input data
        let input_records = DataIO::simulate_read_input_data(&task.input_location).await?;
        let transformation_steps = self.parse_transformation_steps(&task.options)?;
        
        // Apply transformations
        let mut transformed_records = input_records.clone();
        let mut transformation_results = Vec::new();
        
        for (step_index, step) in transformation_steps.iter().enumerate() {
            let step_result = self.apply_transformation_step(&mut transformed_records, step, step_index).await?;
            transformation_results.push(step_result);
        }
        
        // Simulate writing output data
        let output_records = transformed_records.len();
        DataIO::simulate_write_output_data(&task.output_destination, &transformed_records).await?;

        let result = serde_json::json!({
            "pipeline_type": "transformation",
            "pipeline": task.pipeline,
            "input_records": input_records.len(),
            "output_records": output_records,
            "transformation_steps": transformation_steps,
            "step_results": transformation_results,
            "status": "completed"
        });

        Ok(result)
    }
    
    /// Parse transformation steps from options
    fn parse_transformation_steps(&self, options: &HashMap<String, String>) -> Result<Vec<Value>, AgentError> {
        ConfigParser::parse_array_config(options, "steps")
    }
    
    /// Apply a single transformation step
    async fn apply_transformation_step(
        &self, 
        records: &mut Vec<Value>, 
        step: &Value, 
        step_index: usize
    ) -> Result<Value, AgentError> {
        let step_type = step.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
        
        match step_type {
            "rename_field" => {
                let from_field = step.get("from").and_then(|v| v.as_str()).unwrap_or("");
                let to_field = step.get("to").and_then(|v| v.as_str()).unwrap_or("");
                
                for record in records.iter_mut() {
                    if let Some(value) = record.get(from_field).cloned() {
                        record[to_field] = value;
                        record.as_object_mut().unwrap().remove(from_field);
                    }
                }
                
                Ok(serde_json::json!({
                    "step": step_index,
                    "type": "rename_field",
                    "from": from_field,
                    "to": to_field,
                    "records_processed": records.len()
                }))
            }
            "add_field" => {
                let field_name = step.get("field").and_then(|v| v.as_str()).unwrap_or("");
                let field_value = step.get("value").unwrap_or(&serde_json::Value::Null);
                
                for record in records.iter_mut() {
                    record[field_name] = field_value.clone();
                }
                
                Ok(serde_json::json!({
                    "step": step_index,
                    "type": "add_field",
                    "field": field_name,
                    "value": field_value,
                    "records_processed": records.len()
                }))
            }
            "remove_field" => {
                let field_name = step.get("field").and_then(|v| v.as_str()).unwrap_or("");
                
                for record in records.iter_mut() {
                    record.as_object_mut().unwrap().remove(field_name);
                }
                
                Ok(serde_json::json!({
                    "step": step_index,
                    "type": "remove_field",
                    "field": field_name,
                    "records_processed": records.len()
                }))
            }
            "convert_type" => {
                let field_name = step.get("field").and_then(|v| v.as_str()).unwrap_or("");
                let target_type = step.get("target_type").and_then(|v| v.as_str()).unwrap_or("string");
                
                for record in records.iter_mut() {
                    if let Some(value) = record.get(field_name) {
                        let converted_value = match target_type {
                            "string" => serde_json::Value::String(value.to_string()),
                            "number" => {
                                if let Some(num) = value.as_f64() {
                                    serde_json::Value::Number(serde_json::Number::from_f64(num).unwrap_or(serde_json::Number::from(0)))
                                } else {
                                    value.clone()
                                }
                            }
                            "boolean" => {
                                serde_json::Value::Bool(value.as_bool().unwrap_or(false))
                            }
                            _ => value.clone()
                        };
                        record[field_name] = converted_value;
                    }
                }
                
                Ok(serde_json::json!({
                    "step": step_index,
                    "type": "convert_type",
                    "field": field_name,
                    "target_type": target_type,
                    "records_processed": records.len()
                }))
            }
            _ => {
                Ok(serde_json::json!({
                    "step": step_index,
                    "type": "unknown",
                    "error": format!("Unknown transformation type: {}", step_type),
                    "records_processed": 0
                }))
            }
        }
    }
}

impl BaseProcessor for TransformationProcessor {
    fn config(&self) -> &AgentConfig {
        &self.config
    }
    
    fn state(&self) -> &Arc<RwLock<AgentState>> {
        &self.state
    }
}
