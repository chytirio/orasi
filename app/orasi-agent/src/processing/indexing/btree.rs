//! B-tree index builder with S3 support
//! 
//! This module provides functionality to build B-tree indexes from various data sources
//! including local files and Amazon S3. The implementation supports:
//! 
//! - Reading data from S3 buckets (s3://bucket/key format)
//! - Reading data from local files (file://path or direct path)
//! - Support for JSON, JSONL, and CSV file formats
//! - Writing index data back to S3 or local files
//! - Comprehensive error handling and validation
//! 
//! ## S3 Integration
//! 
//! The S3 functionality requires AWS credentials to be configured via:
//! - AWS CLI configuration
//! - Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
//! - IAM roles (when running on EC2)
//! - AWS SSO
//! 
//! ## Usage Example
//! 
//! ```rust
//! use crate::processing::indexing::btree::BTreeIndexBuilder;
//! use crate::config::AgentConfig;
//! 
//! let config = AgentConfig::default();
//! let builder = BTreeIndexBuilder::new(config);
//! 
//! // Build index from S3 data
//! let result = builder.build_index(&task).await?;
//! ```

use super::super::tasks::*;
use crate::types::*;
use crate::{config::AgentConfig, error::AgentError};
use aws_config::BehaviorVersion;
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use tracing::{error, info, warn};

/// B-tree index builder
pub struct BTreeIndexBuilder {
    config: AgentConfig,
}

impl BTreeIndexBuilder {
    /// Create new B-tree index builder
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }

    /// Build B-tree index
    pub async fn build_index(&self, task: &IndexingTask) -> Result<Value, AgentError> {
        info!("Building B-tree index for: {}", task.data_location);

        // Read data from source
        let data = self.read_data_from_source(&task.data_location).await?;

        // Build B-tree index
        let index_data = self
            .build_btree_structure(&data, &task.index_config.fields)
            .await?;

        // Write index to destination
        let index_size = self
            .write_index_to_destination(&index_data, &task.destination)
            .await?;

        // Calculate statistics
        let indexed_records = data.len();
        let optimization_level = self.determine_optimization_level(&index_data).await?;

        let mut result = serde_json::Map::new();
        result.insert(
            "build_status".to_string(),
            Value::String("completed".to_string()),
        );
        result.insert(
            "optimization_level".to_string(),
            Value::String(optimization_level),
        );
        result.insert(
            "tree_height".to_string(),
            Value::Number(serde_json::Number::from(index_data.tree_height)),
        );
        result.insert(
            "leaf_nodes".to_string(),
            Value::Number(serde_json::Number::from(index_data.leaf_count)),
        );
        result.insert(
            "internal_nodes".to_string(),
            Value::Number(serde_json::Number::from(index_data.internal_count)),
        );
        result.insert(
            "total_nodes".to_string(),
            Value::Number(serde_json::Number::from(index_data.node_count)),
        );
        result.insert("index_type".to_string(), Value::String("btree".to_string()));
        result.insert(
            "created_at".to_string(),
            Value::Number(serde_json::Number::from(index_data.created_at)),
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
        let config = aws_config::defaults(BehaviorVersion::latest())
            .load()
            .await;
        let s3_client = S3Client::new(&config);

        info!("S3 client initialized for bucket: {}", bucket);

        // Get object from S3
        let response = s3_client
            .get_object()
            .bucket(bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| {
                AgentError::IoError(format!("Failed to get object from S3: {}", e))
            })?;

        // Read object body
        let body = response.body.collect().await.map_err(|e| {
            AgentError::IoError(format!("Failed to read S3 object body: {}", e))
        })?;

        let content = String::from_utf8(body.into_bytes().to_vec())
            .map_err(|e| AgentError::IoError(format!("Failed to decode S3 object content: {}", e)))?;

        // Parse content based on file extension
        if key.ends_with(".jsonl") {
            info!("Parsing JSONL content from S3");
            self.parse_jsonl(&content)
        } else if key.ends_with(".csv") {
            info!("Parsing CSV content from S3");
            self.parse_csv(&content)
        } else if key.ends_with(".json") {
            info!("Parsing JSON content from S3");
            serde_json::from_str(&content)
                .map_err(|e| AgentError::Serialization(format!("Failed to parse JSON from S3: {}", e)))
        } else {
            // Assume JSON array for unknown extensions
            info!("Parsing content as JSON array from S3 (unknown extension)");
            serde_json::from_str(&content)
                .map_err(|e| AgentError::Serialization(format!("Failed to parse JSON from S3: {}", e)))
        }
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

    /// Build B-tree structure from data
    async fn build_btree_structure(
        &self,
        data: &[HashMap<String, Value>],
        fields: &[String],
    ) -> Result<BTreeIndexData, AgentError> {
        info!("Building B-tree structure for {} records", data.len());

        let mut btree = BTreeMap::new();
        let mut leaf_count = 0;
        let mut internal_count = 0;

        // Build B-tree by inserting records
        for (record_id, record) in data.iter().enumerate() {
            let key = self.extract_key_from_record(record, fields)?;
            btree.insert(key, record_id);
        }

        // Calculate tree statistics
        let height = self.calculate_tree_height(&btree);
        leaf_count = btree.len(); // Simplified: each entry is a leaf
        internal_count = if height > 1 { height - 1 } else { 0 };

        Ok(BTreeIndexData {
            tree_height: height as usize,
            node_count: btree.len(),
            leaf_count,
            internal_count: internal_count as usize,
            optimization_level: self
                .determine_optimization_level(&BTreeIndexData {
                    tree_height: height as usize,
                    node_count: btree.len(),
                    leaf_count,
                    internal_count: internal_count as usize,
                    optimization_level: "".to_string(),
                    index_type: "".to_string(),
                    created_at: 0,
                })
                .await?
                .to_string(),
            index_type: "btree".to_string(),
            created_at: chrono::Utc::now().timestamp() as u64,
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

    /// Calculate tree height
    fn calculate_tree_height(&self, btree: &BTreeMap<String, usize>) -> u32 {
        if btree.is_empty() {
            0
        } else {
            // Simplified height calculation
            // In a real B-tree, this would be more complex
            (btree.len() as f64).log(4.0).ceil() as u32 + 1
        }
    }

    /// Determine optimization level based on index characteristics
    async fn determine_optimization_level(
        &self,
        index_data: &BTreeIndexData,
    ) -> Result<String, AgentError> {
        let total_nodes = index_data.leaf_count + index_data.internal_count;

        let optimization_level = if index_data.tree_height <= 3 {
            "high"
        } else if index_data.tree_height <= 5 {
            "standard"
        } else {
            "basic"
        };

        Ok(optimization_level.to_string())
    }

    /// Write index to destination
    async fn write_index_to_destination(
        &self,
        index_data: &BTreeIndexData,
        destination: &str,
    ) -> Result<u64, AgentError> {
        info!("Writing index to: {}", destination);

        // Serialize index data
        let index_json = serde_json::to_string(&index_data)
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
        let config = aws_config::defaults(BehaviorVersion::latest())
            .load()
            .await;
        let s3_client = S3Client::new(&config);

        info!("S3 client initialized for bucket: {}", bucket);

        // Create byte stream from data
        let body = ByteStream::from(data.as_bytes().to_vec());

        // Upload object to S3
        s3_client
            .put_object()
            .bucket(bucket)
            .key(&key)
            .body(body)
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| {
                AgentError::IoError(format!("Failed to upload object to S3: {}", e))
            })?;

        info!("Successfully uploaded index to S3: {}", s3_location);
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

    /// Parse S3 location into bucket and key
    fn parse_s3_location(&self, s3_location: &str) -> Result<(String, String), AgentError> {
        if !s3_location.starts_with("s3://") {
            return Err(AgentError::InvalidInput(format!(
                "Invalid S3 location format: {}. Expected format: s3://bucket/key",
                s3_location
            )));
        }

        let path = &s3_location[5..]; // Remove "s3://" prefix
        
        // Handle edge cases
        if path.is_empty() {
            return Err(AgentError::InvalidInput(format!(
                "Invalid S3 location: path is empty after removing s3:// prefix. Location: {}",
                s3_location
            )));
        }

        let parts: Vec<&str> = path.splitn(2, '/').collect();

        if parts.len() != 2 {
            return Err(AgentError::InvalidInput(format!(
                "Invalid S3 location format: {}. Expected format: s3://bucket/key",
                s3_location
            )));
        }

        let bucket = parts[0].to_string();
        let key = parts[1].to_string();

        if bucket.is_empty() || key.is_empty() {
            return Err(AgentError::InvalidInput(format!(
                "Invalid S3 location: bucket and key cannot be empty. Location: {}",
                s3_location
            )));
        }

        // Validate bucket name format (basic validation)
        if !bucket.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '.') {
            return Err(AgentError::InvalidInput(format!(
                "Invalid S3 bucket name: {}. Bucket names can only contain lowercase letters, numbers, hyphens, and dots",
                bucket
            )));
        }

        info!("Parsed S3 location - Bucket: {}, Key: {}", bucket, key);
        Ok((bucket, key))
    }
}

/// B-tree index data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BTreeIndexData {
    pub tree_height: usize,
    pub node_count: usize,
    pub leaf_count: usize,
    pub internal_count: usize,
    pub optimization_level: String,
    pub index_type: String,
    pub created_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentConfig;

    #[test]
    fn test_parse_s3_location_valid() {
        let config = AgentConfig::default();
        let builder = BTreeIndexBuilder::new(config);
        
        let result = builder.parse_s3_location("s3://my-bucket/path/to/file.json");
        assert!(result.is_ok());
        
        let (bucket, key) = result.unwrap();
        assert_eq!(bucket, "my-bucket");
        assert_eq!(key, "path/to/file.json");
    }

    #[test]
    fn test_parse_s3_location_invalid_format() {
        let config = AgentConfig::default();
        let builder = BTreeIndexBuilder::new(config);
        
        let result = builder.parse_s3_location("invalid-format");
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Invalid S3 location format"));
    }

    #[test]
    fn test_parse_s3_location_empty_parts() {
        let config = AgentConfig::default();
        let builder = BTreeIndexBuilder::new(config);
        
        let result = builder.parse_s3_location("s3://");
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert!(error.to_string().contains("path is empty"));
    }

    #[test]
    fn test_parse_s3_location_missing_key() {
        let config = AgentConfig::default();
        let builder = BTreeIndexBuilder::new(config);
        
        let result = builder.parse_s3_location("s3://my-bucket");
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Invalid S3 location format"));
    }

    #[test]
    fn test_parse_s3_location_invalid_bucket_name() {
        let config = AgentConfig::default();
        let builder = BTreeIndexBuilder::new(config);
        
        let result = builder.parse_s3_location("s3://MY_BUCKET/file.json");
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Invalid S3 bucket name"));
    }
}
