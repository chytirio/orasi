//! Composite index builder

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::{IndexingTask, TaskResult};
use crate::state::AgentState;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Composite index builder
pub struct CompositeIndexBuilder {
    config: AgentConfig,
}

impl CompositeIndexBuilder {
    /// Create new composite index builder
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }

    /// Build composite index
    pub async fn build_index(&self, task: &IndexingTask) -> Result<Value, AgentError> {
        info!("Building composite index for: {}", task.data_location);

        // Read data from source
        let data = self.read_data_from_source(&task.data_location).await?;

        // Build composite index
        let index_data = self
            .build_composite_structure(&data, &task.index_config.fields)
            .await?;

        // Write index to destination
        let index_size = self
            .write_index_to_destination(&index_data, &task.destination)
            .await?;

        // Calculate statistics
        let indexed_records = data.len();
        let field_order = self.build_field_order(&task.index_config.fields).await?;

        let mut result = serde_json::Map::new();
        result.insert(
            "index_type".to_string(),
            Value::String("composite".to_string()),
        );
        result.insert(
            "data_location".to_string(),
            Value::String(task.data_location.clone()),
        );
        result.insert(
            "destination".to_string(),
            Value::String(task.destination.clone()),
        );
        result.insert(
            "fields".to_string(),
            Value::Array(
                task.index_config
                    .fields
                    .iter()
                    .map(|f| Value::String(f.clone()))
                    .collect(),
            ),
        );
        result.insert(
            "index_size_bytes".to_string(),
            Value::Number(serde_json::Number::from(index_size)),
        );
        result.insert(
            "indexed_records".to_string(),
            Value::Number(serde_json::Number::from(indexed_records)),
        );
        result.insert(
            "build_status".to_string(),
            Value::String("completed".to_string()),
        );
        result.insert("field_order".to_string(), field_order);
        result.insert(
            "key_count".to_string(),
            Value::Number(serde_json::Number::from(index_data.key_count)),
        );
        result.insert(
            "avg_key_length".to_string(),
            Value::Number(
                serde_json::Number::from_f64(index_data.avg_key_length)
                    .unwrap_or(serde_json::Number::from(0)),
            ),
        );

        Ok(Value::Object(result))
    }

    /// Read data from source location
    async fn read_data_from_source(
        &self,
        data_location: &str,
    ) -> Result<Vec<HashMap<String, Value>>, AgentError> {
        info!("Reading data from: {}", data_location);

        // Parse data location to determine source type
        if data_location.starts_with("s3://") {
            self.read_from_s3(data_location).await
        } else if data_location.starts_with("file://") || Path::new(data_location).exists() {
            self.read_from_file(data_location).await
        } else {
            Err(AgentError::InvalidInput(format!(
                "Unsupported data location: {}",
                data_location
            )))
        }
    }

    /// Read data from S3
    async fn read_from_s3(
        &self,
        s3_location: &str,
    ) -> Result<Vec<HashMap<String, Value>>, AgentError> {
        // TODO: Implement S3 data reading
        info!("Reading from S3: {}", s3_location);

        // Mock implementation for now
        let mut data = Vec::new();
        for i in 0..1000 {
            let mut record = HashMap::new();
            record.insert("id".to_string(), Value::Number(serde_json::Number::from(i)));
            record.insert(
                "category".to_string(),
                Value::String(format!("category_{}", i % 10)),
            );
            record.insert(
                "status".to_string(),
                Value::String(format!("status_{}", i % 5)),
            );
            record.insert(
                "priority".to_string(),
                Value::Number(serde_json::Number::from(i % 3)),
            );
            record.insert("name".to_string(), Value::String(format!("record_{}", i)));
            data.push(record);
        }

        Ok(data)
    }

    /// Read data from local file
    async fn read_from_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<HashMap<String, Value>>, AgentError> {
        info!("Reading from file: {}", file_path);

        let path = if file_path.starts_with("file://") {
            &file_path[7..]
        } else {
            file_path
        };

        let content = fs::read_to_string(path)
            .map_err(|e| AgentError::IoError(format!("Failed to read file: {}", e)))?;

        // Parse as JSON lines or CSV based on file extension
        if path.ends_with(".jsonl") {
            self.parse_jsonl(&content)
        } else if path.ends_with(".csv") {
            self.parse_csv(&content)
        } else {
            // Assume JSON array
            serde_json::from_str(&content)
                .map_err(|e| AgentError::Serialization(format!("Failed to parse JSON: {}", e)))
        }
    }

    /// Parse JSONL format
    fn parse_jsonl(&self, content: &str) -> Result<Vec<HashMap<String, Value>>, AgentError> {
        let mut data = Vec::new();
        for line in content.lines() {
            if !line.trim().is_empty() {
                let record: HashMap<String, Value> = serde_json::from_str(line).map_err(|e| {
                    AgentError::Serialization(format!("Failed to parse JSONL line: {}", e))
                })?;
                data.push(record);
            }
        }
        Ok(data)
    }

    /// Parse CSV format
    fn parse_csv(&self, content: &str) -> Result<Vec<HashMap<String, Value>>, AgentError> {
        let mut data = Vec::new();
        let mut lines = content.lines();

        // Read header
        let header = lines
            .next()
            .ok_or_else(|| AgentError::InvalidInput("Empty CSV file".to_string()))?
            .split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>();

        // Read data rows
        for line in lines {
            if !line.trim().is_empty() {
                let values: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if values.len() == header.len() {
                    let mut record = HashMap::new();
                    for (i, field) in header.iter().enumerate() {
                        let value = values[i];
                        // Try to parse as number, otherwise keep as string
                        if let Ok(num) = value.parse::<i64>() {
                            record.insert(
                                field.clone(),
                                Value::Number(serde_json::Number::from(num)),
                            );
                        } else if let Ok(num) = value.parse::<f64>() {
                            record.insert(
                                field.clone(),
                                Value::Number(
                                    serde_json::Number::from_f64(num)
                                        .unwrap_or(serde_json::Number::from(0)),
                                ),
                            );
                        } else {
                            record.insert(field.clone(), Value::String(value.to_string()));
                        }
                    }
                    data.push(record);
                }
            }
        }

        Ok(data)
    }

    /// Build composite structure from data
    async fn build_composite_structure(
        &self,
        data: &[HashMap<String, Value>],
        fields: &[String],
    ) -> Result<CompositeIndexData, AgentError> {
        info!("Building composite structure for {} records", data.len());

        let mut composite_map = BTreeMap::new();
        let mut total_key_length = 0;
        let mut key_count = 0;

        // Build composite index by processing each record
        for (record_id, record) in data.iter().enumerate() {
            let composite_key = self.build_composite_key(record, fields).await?;

            // Store in composite map
            composite_map.insert(composite_key.clone(), record_id);

            total_key_length += composite_key.len();
            key_count += 1;
        }

        let avg_key_length = if key_count > 0 {
            total_key_length as f64 / key_count as f64
        } else {
            0.0
        };

        Ok(CompositeIndexData {
            composite_map,
            key_count,
            avg_key_length,
        })
    }

    /// Build composite key from record
    async fn build_composite_key(
        &self,
        record: &HashMap<String, Value>,
        fields: &[String],
    ) -> Result<String, AgentError> {
        let mut key_parts = Vec::new();

        for field in fields {
            if let Some(value) = record.get(field) {
                // Format value based on its type
                let formatted_value = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "null".to_string(),
                    _ => value.to_string(),
                };
                key_parts.push(formatted_value);
            } else {
                return Err(AgentError::InvalidInput(format!(
                    "Field '{}' not found in record",
                    field
                )));
            }
        }

        // Join with a separator that's unlikely to appear in the data
        Ok(key_parts.join("|"))
    }

    /// Build field order information
    async fn build_field_order(&self, fields: &[String]) -> Result<Value, AgentError> {
        let field_order: Vec<Value> = fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let mut obj = HashMap::new();
                obj.insert("field".to_string(), Value::String(field.clone()));
                obj.insert(
                    "order".to_string(),
                    Value::Number(serde_json::Number::from(i)),
                );
                Value::Object(serde_json::Map::from_iter(obj.into_iter()))
            })
            .collect();

        Ok(Value::Array(field_order))
    }

    /// Write index to destination
    async fn write_index_to_destination(
        &self,
        index_data: &CompositeIndexData,
        destination: &str,
    ) -> Result<u64, AgentError> {
        info!("Writing composite index to: {}", destination);

        // Serialize index data
        let index_json = serde_json::to_string(&index_data.composite_map)
            .map_err(|e| AgentError::Serialization(format!("Failed to serialize index: {}", e)))?;

        // Write to destination
        if destination.starts_with("s3://") {
            self.write_to_s3(&index_json, destination).await
        } else {
            self.write_to_file(&index_json, destination).await
        }
    }

    /// Write to S3
    async fn write_to_s3(&self, data: &str, s3_location: &str) -> Result<u64, AgentError> {
        // TODO: Implement S3 writing
        info!("Writing to S3: {}", s3_location);

        // Mock implementation
        Ok(data.len() as u64)
    }

    /// Write to local file
    async fn write_to_file(&self, data: &str, file_path: &str) -> Result<u64, AgentError> {
        let path = if file_path.starts_with("file://") {
            &file_path[7..]
        } else {
            file_path
        };

        // Ensure directory exists
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AgentError::IoError(format!("Failed to create directory: {}", e)))?;
        }

        fs::write(path, data)
            .map_err(|e| AgentError::IoError(format!("Failed to write file: {}", e)))?;

        Ok(data.len() as u64)
    }
}

/// Composite index data structure
#[derive(Debug, Serialize, Deserialize)]
struct CompositeIndexData {
    composite_map: BTreeMap<String, usize>,
    key_count: usize,
    avg_key_length: f64,
}
