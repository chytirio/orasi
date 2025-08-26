//! Spatial index builder

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::{IndexConfig, IndexingTask, TaskResult};
use crate::state::AgentState;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
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

/// Spatial index builder
pub struct SpatialIndexBuilder {
    config: AgentConfig,
}

impl SpatialIndexBuilder {
    /// Create new spatial index builder
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }

    /// Build spatial index
    pub async fn build_index(&self, task: &IndexingTask) -> Result<Value, AgentError> {
        info!("Building spatial index for: {}", task.data_location);

        // Read data from source
        let data = self.read_data_from_source(&task.data_location).await?;

        // Build spatial index
        let index_data = self
            .build_spatial_structure(&data, &task.index_config.fields)
            .await?;

        // Write index to destination
        let index_size = self
            .write_index_to_destination(&index_data, &task.destination)
            .await?;

        // Calculate statistics
        let indexed_records = data.len();
        let spatial_type = self.determine_spatial_type(&task.index_config).await?;
        let bounding_box = self.calculate_bounding_box(&index_data).await?;

        let mut result = serde_json::Map::new();
        result.insert(
            "index_type".to_string(),
            Value::String("spatial".to_string()),
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
        result.insert("spatial_type".to_string(), Value::String(spatial_type));
        result.insert("bounding_box".to_string(), Value::String(bounding_box));
        result.insert(
            "node_count".to_string(),
            Value::Number(serde_json::Number::from(index_data.node_count)),
        );
        result.insert(
            "max_depth".to_string(),
            Value::Number(serde_json::Number::from(index_data.max_depth)),
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

    /// Build spatial structure from data
    async fn build_spatial_structure(
        &self,
        data: &[HashMap<String, Value>],
        fields: &[String],
    ) -> Result<SpatialIndexData, AgentError> {
        info!("Building spatial structure for {} records", data.len());

        let mut spatial_nodes = Vec::new();
        let mut node_count = 0;
        let mut max_depth = 0;

        // Build spatial index by processing each record
        for (record_id, record) in data.iter().enumerate() {
            let coordinates = self.extract_coordinates(record, fields).await?;

            // Create spatial node
            let node = SpatialNode {
                id: record_id,
                coordinates: coordinates.clone(),
                bounds: self.calculate_bounds(&coordinates),
            };

            spatial_nodes.push(node);
            node_count += 1;
        }

        // Build R-tree structure
        let rtree = self.build_rtree(&spatial_nodes).await?;
        max_depth = rtree.max_depth;

        Ok(SpatialIndexData {
            nodes: spatial_nodes,
            rtree,
            node_count,
            max_depth,
        })
    }

    /// Extract coordinates from record
    async fn extract_coordinates(
        &self,
        record: &HashMap<String, Value>,
        fields: &[String],
    ) -> Result<Vec<f64>, AgentError> {
        let mut coordinates = Vec::new();

        for field in fields {
            if let Some(value) = record.get(field) {
                if let Some(num) = value.as_f64() {
                    coordinates.push(num);
                } else {
                    return Err(AgentError::InvalidInput(format!(
                        "Field '{}' is not a valid coordinate",
                        field
                    )));
                }
            } else {
                return Err(AgentError::InvalidInput(format!(
                    "Field '{}' not found in record",
                    field
                )));
            }
        }

        Ok(coordinates)
    }

    /// Calculate bounds for coordinates
    fn calculate_bounds(&self, coordinates: &[f64]) -> BoundingBox {
        if coordinates.len() >= 2 {
            let min_x = coordinates[0];
            let min_y = coordinates[1];
            let max_x = coordinates[0];
            let max_y = coordinates[1];

            BoundingBox {
                min_x,
                min_y,
                max_x,
                max_y,
            }
        } else {
            BoundingBox {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 0.0,
                max_y: 0.0,
            }
        }
    }

    /// Build R-tree structure
    async fn build_rtree(&self, nodes: &[SpatialNode]) -> Result<RTree, AgentError> {
        info!("Building R-tree with {} nodes", nodes.len());

        // Simplified R-tree implementation
        let mut rtree = RTree {
            nodes: nodes.to_vec(),
            max_depth: 0,
        };

        // Calculate max depth based on number of nodes
        if !nodes.is_empty() {
            rtree.max_depth = (nodes.len() as f64).log(4.0).ceil() as u32;
        }

        Ok(rtree)
    }

    /// Determine spatial type based on configuration
    async fn determine_spatial_type(
        &self,
        index_config: &IndexConfig,
    ) -> Result<String, AgentError> {
        // Check if spatial type is specified in options
        if let Some(spatial_type) = index_config.options.get("spatial_type") {
            Ok(spatial_type.clone())
        } else {
            // Default to R-tree
            Ok("r-tree".to_string())
        }
    }

    /// Calculate bounding box for all data
    async fn calculate_bounding_box(
        &self,
        index_data: &SpatialIndexData,
    ) -> Result<String, AgentError> {
        if index_data.nodes.is_empty() {
            return Ok("[]".to_string());
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for node in &index_data.nodes {
            min_x = min_x.min(node.bounds.min_x);
            min_y = min_y.min(node.bounds.min_y);
            max_x = max_x.max(node.bounds.max_x);
            max_y = max_y.max(node.bounds.max_y);
        }

        Ok(format!("[{},{},{},{}]", min_x, min_y, max_x, max_y))
    }

    /// Write index to destination
    async fn write_index_to_destination(
        &self,
        index_data: &SpatialIndexData,
        destination: &str,
    ) -> Result<u64, AgentError> {
        info!("Writing spatial index to: {}", destination);

        // Serialize index data
        let index_json = serde_json::to_string(&index_data.nodes)
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

/// Spatial index data structure
#[derive(Debug, Serialize, Deserialize)]
struct SpatialIndexData {
    nodes: Vec<SpatialNode>,
    rtree: RTree,
    node_count: usize,
    max_depth: u32,
}

/// Spatial node
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpatialNode {
    id: usize,
    coordinates: Vec<f64>,
    bounds: BoundingBox,
}

/// Bounding box
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BoundingBox {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

/// R-tree structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RTree {
    nodes: Vec<SpatialNode>,
    max_depth: u32,
}
