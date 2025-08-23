//! Full-text index builder

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::{IndexConfig, IndexingTask, TaskResult};
use crate::state::AgentState;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Full-text index builder
pub struct FullTextIndexBuilder {
    config: AgentConfig,
}

impl FullTextIndexBuilder {
    /// Create new full-text index builder
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }

    /// Build full-text index
    pub async fn build_index(&self, task: &IndexingTask) -> Result<Value, AgentError> {
        info!("Building full-text index for: {}", task.data_location);

        // Read data from source
        let data = self.read_data_from_source(&task.data_location).await?;

        // Build full-text index
        let index_data = self
            .build_fulltext_structure(&data, &task.index_config.fields)
            .await?;

        // Write index to destination
        let index_size = self
            .write_index_to_destination(&index_data, &task.destination)
            .await?;

        // Calculate statistics
        let indexed_records = data.len();
        let analyzer_type = self.determine_analyzer_type(&task.index_config).await?;
        let token_count = index_data.token_count;

        let mut result = serde_json::Map::new();
        result.insert(
            "index_type".to_string(),
            Value::String("fulltext".to_string()),
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
        result.insert("analyzer_type".to_string(), Value::String(analyzer_type));
        result.insert(
            "token_count".to_string(),
            Value::Number(serde_json::Number::from(token_count)),
        );
        result.insert(
            "unique_terms".to_string(),
            Value::Number(serde_json::Number::from(index_data.unique_terms)),
        );
        result.insert(
            "avg_term_length".to_string(),
            Value::Number(
                serde_json::Number::from_f64(index_data.avg_term_length)
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
                "title".to_string(),
                Value::String(format!("Document Title {}", i)),
            );
            record.insert("content".to_string(), Value::String(format!("This is the content of document {}. It contains various words and phrases for full-text indexing.", i)));
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

    /// Build full-text structure from data
    async fn build_fulltext_structure(
        &self,
        data: &[HashMap<String, Value>],
        fields: &[String],
    ) -> Result<FullTextIndexData, AgentError> {
        info!("Building full-text structure for {} records", data.len());

        let mut inverted_index = HashMap::new();
        let mut unique_terms = HashSet::new();
        let mut total_term_length = 0;
        let mut total_terms = 0;

        // Build inverted index by processing each record
        for (record_id, record) in data.iter().enumerate() {
            for field in fields {
                if let Some(value) = record.get(field) {
                    let text = value.to_string();
                    let terms = self.tokenize_text(&text).await?;

                    for term in terms {
                        unique_terms.insert(term.clone());
                        total_term_length += term.len();
                        total_terms += 1;

                        // Add to inverted index
                        inverted_index
                            .entry(term)
                            .or_insert_with(Vec::new)
                            .push(record_id);
                    }
                }
            }
        }

        let avg_term_length = if total_terms > 0 {
            total_term_length as f64 / total_terms as f64
        } else {
            0.0
        };

        Ok(FullTextIndexData {
            inverted_index,
            unique_terms: unique_terms.len(),
            token_count: total_terms,
            avg_term_length,
        })
    }

    /// Tokenize text into terms
    async fn tokenize_text(&self, text: &str) -> Result<Vec<String>, AgentError> {
        // Convert to lowercase
        let text = text.to_lowercase();

        // Remove punctuation and split into words
        let re = Regex::new(r"[^\w\s]")
            .map_err(|e| AgentError::InvalidInput(format!("Invalid regex: {}", e)))?;
        let cleaned_text = re.replace_all(&text, " ");

        // Split into words and filter out stop words
        let terms: Vec<String> = cleaned_text
            .split_whitespace()
            .filter(|word| !self.is_stop_word(word))
            .filter(|word| word.len() > 2) // Filter out very short words
            .map(|word| word.to_string())
            .collect();

        Ok(terms)
    }

    /// Check if word is a stop word
    fn is_stop_word(&self, word: &str) -> bool {
        let stop_words = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
            "by", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do",
            "does", "did", "will", "would", "could", "should", "may", "might", "must", "can",
            "this", "that", "these", "those",
        ];

        stop_words.contains(&word)
    }

    /// Determine analyzer type based on configuration
    async fn determine_analyzer_type(
        &self,
        index_config: &IndexConfig,
    ) -> Result<String, AgentError> {
        // Check if analyzer type is specified in options
        if let Some(analyzer) = index_config.options.get("analyzer_type") {
            Ok(analyzer.clone())
        } else {
            // Default to standard analyzer
            Ok("standard".to_string())
        }
    }

    /// Write index to destination
    async fn write_index_to_destination(
        &self,
        index_data: &FullTextIndexData,
        destination: &str,
    ) -> Result<u64, AgentError> {
        info!("Writing full-text index to: {}", destination);

        // Serialize index data
        let index_json = serde_json::to_string(&index_data.inverted_index)
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

/// Full-text index data structure
#[derive(Debug, Serialize, Deserialize)]
struct FullTextIndexData {
    inverted_index: HashMap<String, Vec<usize>>,
    unique_terms: usize,
    token_count: usize,
    avg_term_length: f64,
}
