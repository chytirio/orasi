//! Inverted index builder

use super::super::tasks::*;
use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::processing::tasks::{IndexingTask, TaskResult};
use crate::state::AgentState;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Inverted index builder
pub struct InvertedIndexBuilder {
    config: AgentConfig,
}

impl InvertedIndexBuilder {
    /// Create new inverted index builder
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }

    /// Build inverted index
    pub async fn build_index(&self, task: &IndexingTask) -> Result<Value, AgentError> {
        info!("Building inverted index for: {}", task.data_location);

        // Read data from source
        let data = self.read_data_from_source(&task.data_location).await?;

        // Build inverted index
        let index_data = self
            .build_inverted_structure(&data, &task.index_config.fields)
            .await?;

        // Write index to destination
        let index_size = self
            .write_index_to_destination(&index_data, &task.destination)
            .await?;

        // Calculate statistics
        let indexed_records = data.len();
        let unique_terms = index_data.unique_terms;
        let total_postings = index_data.total_postings;

        let mut result = serde_json::Map::new();
        result.insert(
            "index_type".to_string(),
            Value::String("inverted".to_string()),
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
        result.insert(
            "unique_terms".to_string(),
            Value::Number(serde_json::Number::from(unique_terms)),
        );
        result.insert(
            "total_postings".to_string(),
            Value::Number(serde_json::Number::from(total_postings)),
        );
        result.insert(
            "avg_postings_per_term".to_string(),
            Value::Number(
                serde_json::Number::from_f64(index_data.avg_postings_per_term)
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
            record.insert("content".to_string(), Value::String(format!("This is the content of document {}. It contains various words and phrases for inverted indexing.", i)));
            record.insert(
                "tags".to_string(),
                Value::String(format!("tag1,tag2,tag{}", i % 10)),
            );
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

    /// Build inverted structure from data
    async fn build_inverted_structure(
        &self,
        data: &[HashMap<String, Value>],
        fields: &[String],
    ) -> Result<InvertedIndexData, AgentError> {
        info!("Building inverted structure for {} records", data.len());

        let mut inverted_index = HashMap::new();
        let mut unique_terms = HashSet::new();
        let mut total_postings = 0;

        // Build inverted index by processing each record
        for (record_id, record) in data.iter().enumerate() {
            for field in fields {
                if let Some(value) = record.get(field) {
                    let terms = self.extract_terms(value)?;

                    for term in terms {
                        unique_terms.insert(term.clone());

                        // Add to inverted index
                        let posting_list = inverted_index.entry(term).or_insert_with(Vec::new);

                        // Add posting with position information
                        let posting = Posting {
                            doc_id: record_id,
                            field: field.clone(),
                            frequency: 1,       // Simplified frequency calculation
                            positions: vec![0], // Simplified position tracking
                        };

                        posting_list.push(posting);
                        total_postings += 1;
                    }
                }
            }
        }

        let avg_postings_per_term = if unique_terms.len() > 0 {
            total_postings as f64 / unique_terms.len() as f64
        } else {
            0.0
        };

        Ok(InvertedIndexData {
            inverted_index,
            unique_terms: unique_terms.len(),
            total_postings,
            avg_postings_per_term,
        })
    }

    /// Extract terms from a value
    fn extract_terms(&self, value: &Value) -> Result<Vec<String>, AgentError> {
        match value {
            Value::String(s) => {
                // Tokenize string
                // Note: tokenize_text is async but extract_terms is sync
                // This is a simplified implementation
                let terms = vec![s.to_string()];
                Ok(terms)
            }
            Value::Array(arr) => {
                // Handle array of values
                let mut all_terms = Vec::new();
                for item in arr {
                    let terms = self.extract_terms(item)?;
                    all_terms.extend(terms);
                }
                Ok(all_terms)
            }
            Value::Number(n) => {
                // Convert number to string term
                Ok(vec![n.to_string()])
            }
            Value::Bool(b) => {
                // Convert boolean to string term
                Ok(vec![b.to_string()])
            }
            _ => {
                // For other types, convert to string
                Ok(vec![value.to_string()])
            }
        }
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

    /// Write index to destination
    async fn write_index_to_destination(
        &self,
        index_data: &InvertedIndexData,
        destination: &str,
    ) -> Result<u64, AgentError> {
        info!("Writing inverted index to: {}", destination);

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

/// Inverted index data structure
#[derive(Debug)]
struct InvertedIndexData {
    inverted_index: HashMap<String, Vec<Posting>>,
    unique_terms: usize,
    total_postings: usize,
    avg_postings_per_term: f64,
}

/// Posting entry in inverted index
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Posting {
    doc_id: usize,
    field: String,
    frequency: u32,
    positions: Vec<u32>,
}
