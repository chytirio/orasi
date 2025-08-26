//! Hash index builder

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::{IndexConfig, IndexingTask, TaskResult};
use crate::state::AgentState;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

// AWS SDK imports for S3 support
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;

/// Hash index builder
pub struct HashIndexBuilder {
    config: AgentConfig,
}

impl HashIndexBuilder {
    /// Create new hash index builder
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }

    /// Build hash index
    pub async fn build_index(&self, task: &IndexingTask) -> Result<Value, AgentError> {
        info!("Building hash index for: {}", task.data_location);

        // Read data from source
        let data = self.read_data_from_source(&task.data_location).await?;

        // Build hash index
        let index_data = self
            .build_hash_structure(&data, &task.index_config.fields)
            .await?;

        // Write index to destination
        let index_size = self
            .write_index_to_destination(&index_data, &task.destination)
            .await?;

        // Calculate statistics
        let indexed_records = data.len();
        let hash_function = self.determine_hash_function(&task.index_config).await?;

        let mut result = serde_json::Map::new();
        result.insert("index_type".to_string(), Value::String("hash".to_string()));
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
        result.insert("hash_function".to_string(), Value::String(hash_function));
        result.insert(
            "bucket_count".to_string(),
            Value::Number(serde_json::Number::from(index_data.bucket_count)),
        );
        result.insert(
            "collision_count".to_string(),
            Value::Number(serde_json::Number::from(index_data.collision_count)),
        );
        result.insert(
            "load_factor".to_string(),
            Value::Number(
                serde_json::Number::from_f64(index_data.load_factor)
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
        info!("Reading from S3: {}", s3_location);

        // Parse S3 location (s3://bucket/key)
        let (bucket, key) = self.parse_s3_location(s3_location)?;

        // Initialize AWS config and S3 client
        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .load()
            .await;

        let s3_client = S3Client::new(&aws_config);

        // Get object from S3
        let response = s3_client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                AgentError::Storage(format!("Failed to get object from S3: {}", e))
            })?;

        // Read object body
        let object_data = response.body.collect().await.map_err(|e| {
            AgentError::Storage(format!("Failed to read S3 object body: {}", e))
        })?;

        let content = String::from_utf8(object_data.to_vec()).map_err(|e| {
            AgentError::Storage(format!("Failed to convert S3 object to string: {}", e))
        })?;

        // Parse content based on file extension
        if s3_location.ends_with(".jsonl") {
            self.parse_jsonl(&content)
        } else if s3_location.ends_with(".csv") {
            self.parse_csv(&content)
        } else {
            // Assume JSON array
            serde_json::from_str(&content)
                .map_err(|e| AgentError::Serialization(format!("Failed to parse JSON from S3: {}", e)))
        }
    }

    /// Parse S3 location to extract bucket and key
    fn parse_s3_location(&self, s3_location: &str) -> Result<(String, String), AgentError> {
        if !s3_location.starts_with("s3://") {
            return Err(AgentError::InvalidInput(format!(
                "Invalid S3 location format: {}",
                s3_location
            )));
        }

        let path = &s3_location[5..]; // Remove "s3://" prefix
        let parts: Vec<&str> = path.splitn(2, '/').collect();

        if parts.len() != 2 {
            return Err(AgentError::InvalidInput(format!(
                "Invalid S3 location format: {}",
                s3_location
            )));
        }

        let bucket = parts[0].to_string();
        let key = parts[1].to_string();

        if bucket.is_empty() || key.is_empty() {
            return Err(AgentError::InvalidInput(format!(
                "Invalid S3 location format: {}",
                s3_location
            )));
        }

        Ok((bucket, key))
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

    /// Build hash structure from data
    async fn build_hash_structure(
        &self,
        data: &[HashMap<String, Value>],
        fields: &[String],
    ) -> Result<HashIndexData, AgentError> {
        info!("Building hash structure for {} records", data.len());

        let mut hash_map = HashMap::new();
        let mut collision_count = 0;
        let mut unique_keys = HashSet::new();

        // Build hash index by inserting records
        for (record_id, record) in data.iter().enumerate() {
            let key = self.extract_key_from_record(record, fields)?;
            let hash = self.compute_hash(&key);

            // Check for collisions
            if hash_map.contains_key(&hash) {
                collision_count += 1;
            }

            hash_map.insert(hash, record_id);
            unique_keys.insert(key);
        }

        let bucket_count = hash_map.len();
        let load_factor = data.len() as f64 / bucket_count as f64;

        Ok(HashIndexData {
            hash_map,
            bucket_count,
            collision_count,
            load_factor,
            unique_keys: unique_keys.len(),
        })
    }

    /// Extract key from record based on indexed fields
    fn extract_key_from_record(
        &self,
        record: &HashMap<String, Value>,
        fields: &[String],
    ) -> Result<String, AgentError> {
        let mut key_parts = Vec::new();

        for field in fields {
            if let Some(value) = record.get(field) {
                key_parts.push(value.to_string());
            } else {
                return Err(AgentError::InvalidInput(format!(
                    "Field '{}' not found in record",
                    field
                )));
            }
        }

        Ok(key_parts.join("|"))
    }

    /// Compute hash of key
    fn compute_hash(&self, key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Determine hash function based on configuration
    async fn determine_hash_function(
        &self,
        index_config: &IndexConfig,
    ) -> Result<String, AgentError> {
        // Check if hash function is specified in options
        if let Some(hash_func) = index_config.options.get("hash_function") {
            Ok(hash_func.clone())
        } else {
            // Default to SHA256
            Ok("sha256".to_string())
        }
    }

    /// Write index to destination
    async fn write_index_to_destination(
        &self,
        index_data: &HashIndexData,
        destination: &str,
    ) -> Result<u64, AgentError> {
        info!("Writing hash index to: {}", destination);

        // Serialize index data
        let index_json = serde_json::to_string(&index_data.hash_map)
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
        info!("Writing to S3: {}", s3_location);

        // Parse S3 location (s3://bucket/key)
        let (bucket, key) = self.parse_s3_location(s3_location)?;

        // Initialize AWS config and S3 client
        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .load()
            .await;

        let s3_client = S3Client::new(&aws_config);

        // Create byte stream from data
        let body = ByteStream::from(data.as_bytes().to_vec());

        // Upload object to S3
        s3_client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(body)
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| {
                AgentError::Storage(format!("Failed to upload object to S3: {}", e))
            })?;

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

/// Hash index data structure
#[derive(Debug, Serialize, Deserialize)]
struct HashIndexData {
    hash_map: HashMap<String, usize>,
    bucket_count: usize,
    collision_count: usize,
    load_factor: f64,
    unique_keys: usize,
}
