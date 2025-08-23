//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Transform processor for the OpenTelemetry Data Lake Bridge
//!
//! This module provides a transform processor that can transform telemetry data
//! by applying various transformations such as field mapping, data conversion,
//! and enrichment.

use crate::error::BridgeResult;
use crate::health::checker::HealthCheckCallback;
use crate::health::types::HealthCheckResult;
use crate::traits::processor::TelemetryProcessor;
use crate::types::{
    ProcessedBatch, ProcessedRecord, ProcessingStatus, TelemetryBatch, TelemetryData,
    TelemetryRecord,
};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Transformation operation types
#[derive(Debug, Clone)]
pub enum TransformOperation {
    /// Map field from one name to another
    FieldMapping { from: String, to: String },
    /// Add a constant attribute
    AddAttribute { key: String, value: String },
    /// Remove an attribute
    RemoveAttribute { key: String },
    /// Set field value based on condition
    ConditionalSet {
        field: String,
        condition: String,
        value: String,
    },
    /// Extract value from JSON field
    JsonExtract {
        source_field: String,
        json_path: String,
        target_field: String,
    },
    /// Format timestamp
    TimestampFormat { field: String, format: String },
    /// Convert data type
    TypeConversion {
        field: String,
        target_type: DataType,
    },
    /// Enrich with external data
    Enrichment { source: String, fields: Vec<String> },
}

/// Data types for type conversion
#[derive(Debug, Clone)]
pub enum DataType {
    String,
    Integer,
    Float,
    Boolean,
    Timestamp,
}

/// Transform processor configuration
#[derive(Debug, Clone)]
pub struct TransformProcessorConfig {
    /// Transformations to apply
    pub transformations: Vec<TransformOperation>,

    /// Whether to continue processing on transformation errors
    pub continue_on_error: bool,

    /// Maximum number of transformations per record
    pub max_transformations_per_record: usize,

    /// Enable transformation caching
    pub enable_caching: bool,

    /// Cache size limit
    pub cache_size_limit: usize,
}

impl Default for TransformProcessorConfig {
    fn default() -> Self {
        Self {
            transformations: Vec::new(),
            continue_on_error: true,
            max_transformations_per_record: 100,
            enable_caching: false,
            cache_size_limit: 1000,
        }
    }
}

/// Transform processor statistics
#[derive(Debug, Clone)]
pub struct TransformProcessorStats {
    /// Total records processed
    pub total_records: u64,

    /// Total transformations applied
    pub total_transformations: u64,

    /// Total transformation errors
    pub transformation_errors: u64,

    /// Average processing time per record in milliseconds
    pub avg_processing_time_ms: f64,

    /// Cache hits (if caching enabled)
    pub cache_hits: u64,

    /// Cache misses (if caching enabled)
    pub cache_misses: u64,

    /// Records per minute
    pub records_per_minute: u64,

    /// Last processing time
    pub last_processing_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for TransformProcessorStats {
    fn default() -> Self {
        Self {
            total_records: 0,
            total_transformations: 0,
            transformation_errors: 0,
            avg_processing_time_ms: 0.0,
            cache_hits: 0,
            cache_misses: 0,
            records_per_minute: 0,
            last_processing_time: None,
        }
    }
}

/// Transform processor for transforming telemetry data
#[derive(Debug)]
pub struct TransformProcessor {
    /// Processor configuration
    pub config: TransformProcessorConfig,

    /// Processor statistics
    pub stats: Arc<RwLock<TransformProcessorStats>>,

    /// Running state
    pub running: Arc<RwLock<bool>>,

    /// Transformation cache
    pub cache: Arc<RwLock<HashMap<String, String>>>,
}

impl TransformProcessor {
    /// Create a new transform processor
    pub fn new(config: TransformProcessorConfig) -> Self {
        info!(
            "Creating transform processor with {} transformations",
            config.transformations.len()
        );

        Self {
            config,
            stats: Arc::new(RwLock::new(TransformProcessorStats::default())),
            running: Arc::new(RwLock::new(false)),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the transform processor
    pub async fn start(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(crate::error::BridgeError::internal(
                "Transform processor is already running",
            ));
        }

        *running = true;
        info!("Starting transform processor");

        Ok(())
    }

    /// Stop the transform processor
    pub async fn stop(&self) -> BridgeResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Transform processor is not running",
            ));
        }

        *running = false;
        info!("Stopping transform processor");

        Ok(())
    }

    /// Apply transformations to a single record
    async fn transform_record(&self, record: &TelemetryRecord) -> BridgeResult<ProcessedRecord> {
        let start_time = std::time::Instant::now();
        let mut transformed_data = record.data.clone();
        let mut attributes = record.attributes.clone();
        let mut transformation_count = 0;
        let mut errors = Vec::new();

        for transformation in &self.config.transformations {
            if transformation_count >= self.config.max_transformations_per_record {
                warn!(
                    "Maximum transformations per record reached: {}",
                    self.config.max_transformations_per_record
                );
                break;
            }

            match self
                .apply_transformation(transformation, &mut transformed_data, &mut attributes)
                .await
            {
                Ok(_) => {
                    transformation_count += 1;
                    debug!("Applied transformation: {:?}", transformation);
                }
                Err(e) => {
                    let error_msg = format!("Transformation failed: {:?} - {}", transformation, e);
                    errors.push(crate::types::ProcessingError {
                        code: "TRANSFORM_ERROR".to_string(),
                        message: error_msg.clone(),
                        details: None,
                    });

                    if !self.config.continue_on_error {
                        return Ok(ProcessedRecord {
                            original_id: record.id,
                            status: ProcessingStatus::Failed,
                            transformed_data: None,
                            metadata: HashMap::from([
                                (
                                    "transformation_count".to_string(),
                                    transformation_count.to_string(),
                                ),
                                ("error".to_string(), error_msg),
                            ]),
                            errors,
                        });
                    }
                }
            }
        }

        // Update statistics
        let duration = start_time.elapsed();
        {
            let mut stats = self.stats.write().await;
            stats.total_records += 1;
            stats.total_transformations += transformation_count as u64;
            stats.transformation_errors += errors.len() as u64;
            stats.last_processing_time = Some(chrono::Utc::now());

            // Update average processing time
            if stats.total_records > 0 {
                stats.avg_processing_time_ms = (stats.avg_processing_time_ms
                    * (stats.total_records - 1) as f64
                    + duration.as_millis() as f64)
                    / stats.total_records as f64;
            }
        }

        // Create processed record with transformed data
        let mut processed_record = ProcessedRecord {
            original_id: record.id,
            status: if errors.is_empty() {
                ProcessingStatus::Success
            } else {
                ProcessingStatus::Partial
            },
            transformed_data: Some(transformed_data),
            metadata: HashMap::from([
                (
                    "transformation_count".to_string(),
                    transformation_count.to_string(),
                ),
                (
                    "processing_time_ms".to_string(),
                    duration.as_millis().to_string(),
                ),
            ]),
            errors,
        };

        // Add transformed attributes to metadata
        for (key, value) in attributes {
            processed_record
                .metadata
                .insert(format!("attr_{}", key), value);
        }

        Ok(processed_record)
    }

    /// Apply a single transformation
    async fn apply_transformation(
        &self,
        transformation: &TransformOperation,
        data: &mut TelemetryData,
        attributes: &mut HashMap<String, String>,
    ) -> BridgeResult<()> {
        match transformation {
            TransformOperation::FieldMapping { from, to } => {
                if let Some(value) = attributes.remove(from) {
                    attributes.insert(to.clone(), value);
                }
            }

            TransformOperation::AddAttribute { key, value } => {
                attributes.insert(key.clone(), value.clone());
            }

            TransformOperation::RemoveAttribute { key } => {
                attributes.remove(key);
            }

            TransformOperation::ConditionalSet {
                field,
                condition,
                value,
            } => {
                if self.evaluate_condition(condition, data, attributes).await? {
                    attributes.insert(field.clone(), value.clone());
                }
            }

            TransformOperation::JsonExtract {
                source_field,
                json_path,
                target_field,
            } => {
                if let Some(json_str) = attributes.get(source_field) {
                    if let Ok(json_value) = serde_json::from_str::<Value>(json_str) {
                        if let Some(extracted_value) =
                            self.extract_json_path(&json_value, json_path)
                        {
                            attributes.insert(target_field.clone(), extracted_value.to_string());
                        }
                    }
                }
            }

            TransformOperation::TimestampFormat { field, format } => {
                if let Some(timestamp_str) = attributes.get(field) {
                    if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(timestamp_str) {
                        let formatted = timestamp.format(format).to_string();
                        attributes.insert(field.clone(), formatted);
                    }
                }
            }

            TransformOperation::TypeConversion { field, target_type } => {
                if let Some(value) = attributes.get(field) {
                    let converted = self.convert_type(value, target_type)?;
                    attributes.insert(field.clone(), converted);
                }
            }

            TransformOperation::Enrichment { source, fields } => {
                // Placeholder for enrichment logic
                // In a real implementation, this would query external data sources
                for field in fields {
                    attributes.insert(
                        format!("enriched_{}", field),
                        format!("enriched_from_{}", source),
                    );
                }
            }
        }

        Ok(())
    }

    /// Evaluate a condition string
    async fn evaluate_condition(
        &self,
        condition: &str,
        _data: &TelemetryData,
        attributes: &HashMap<String, String>,
    ) -> BridgeResult<bool> {
        // Simple condition evaluation - in practice, this could use a more sophisticated expression parser
        if condition.contains("=") {
            let parts: Vec<&str> = condition.split('=').collect();
            if parts.len() == 2 {
                let field = parts[0].trim();
                let expected_value = parts[1].trim().trim_matches('"');
                return Ok(attributes.get(field).map_or(false, |v| v == expected_value));
            }
        }

        // Default to false for unknown conditions
        Ok(false)
    }

    /// Extract value from JSON using a simple path
    fn extract_json_path(&self, json: &Value, path: &str) -> Option<Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = json;

        for part in parts {
            if let Some(value) = current.get(part) {
                current = value;
            } else {
                return None;
            }
        }

        Some(current.clone())
    }

    /// Convert value to target type
    fn convert_type(&self, value: &str, target_type: &DataType) -> BridgeResult<String> {
        match target_type {
            DataType::String => Ok(value.to_string()),
            DataType::Integer => value.parse::<i64>().map(|i| i.to_string()).map_err(|e| {
                crate::error::BridgeError::internal(format!("Failed to convert to integer: {}", e))
            }),
            DataType::Float => value.parse::<f64>().map(|f| f.to_string()).map_err(|e| {
                crate::error::BridgeError::internal(format!("Failed to convert to float: {}", e))
            }),
            DataType::Boolean => value.parse::<bool>().map(|b| b.to_string()).map_err(|e| {
                crate::error::BridgeError::internal(format!("Failed to convert to boolean: {}", e))
            }),
            DataType::Timestamp => chrono::DateTime::parse_from_rfc3339(value)
                .map(|dt| dt.to_rfc3339())
                .map_err(|e| {
                    crate::error::BridgeError::internal(format!(
                        "Failed to convert to timestamp: {}",
                        e
                    ))
                }),
        }
    }
}

#[async_trait]
impl TelemetryProcessor for TransformProcessor {
    fn name(&self) -> &str {
        "transform_processor"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn process(&self, batch: TelemetryBatch) -> BridgeResult<ProcessedBatch> {
        let running = self.running.read().await;
        if !*running {
            return Err(crate::error::BridgeError::internal(
                "Transform processor is not running",
            ));
        }

        let start_time = std::time::Instant::now();
        let mut processed_records = Vec::new();
        let mut processing_errors = Vec::new();

        for record in &batch.records {
            match self.transform_record(record).await {
                Ok(processed_record) => {
                    processed_records.push(processed_record);
                }
                Err(e) => {
                    processing_errors.push(crate::types::ProcessingError {
                        code: "RECORD_PROCESSING_ERROR".to_string(),
                        message: format!("Failed to process record {}: {}", record.id, e),
                        details: None,
                    });

                    // Add failed record
                    processed_records.push(ProcessedRecord {
                        original_id: record.id,
                        status: ProcessingStatus::Failed,
                        transformed_data: None,
                        metadata: HashMap::from([("error".to_string(), e.to_string())]),
                        errors: vec![crate::types::ProcessingError {
                            code: "PROCESSING_FAILED".to_string(),
                            message: e.to_string(),
                            details: None,
                        }],
                    });
                }
            }
        }

        let processing_status = if processing_errors.is_empty() {
            ProcessingStatus::Success
        } else if processed_records
            .iter()
            .any(|r| r.status == ProcessingStatus::Success)
        {
            ProcessingStatus::Partial
        } else {
            ProcessingStatus::Failed
        };

        let duration = start_time.elapsed();

        Ok(ProcessedBatch {
            original_batch_id: batch.id,
            timestamp: chrono::Utc::now(),
            status: processing_status,
            records: processed_records,
            metadata: HashMap::from([
                ("processor".to_string(), "transform".to_string()),
                (
                    "processing_time_ms".to_string(),
                    duration.as_millis().to_string(),
                ),
                (
                    "transformation_count".to_string(),
                    self.config.transformations.len().to_string(),
                ),
            ]),
            errors: processing_errors,
        })
    }

    async fn get_stats(&self) -> BridgeResult<crate::traits::processor::ProcessorStats> {
        let stats = self.stats.read().await;
        Ok(crate::traits::processor::ProcessorStats {
            total_batches: 0, // Will be tracked by pipeline
            total_records: stats.total_records,
            batches_per_minute: 0, // Will be tracked by pipeline
            records_per_minute: stats.records_per_minute,
            avg_processing_time_ms: stats.avg_processing_time_ms,
            error_count: stats.transformation_errors,
            last_process_time: stats.last_processing_time,
        })
    }

    async fn health_check(&self) -> BridgeResult<bool> {
        let running = self.running.read().await;
        Ok(*running)
    }

    async fn shutdown(&self) -> BridgeResult<()> {
        self.stop().await
    }
}

#[async_trait]
impl HealthCheckCallback for TransformProcessor {
    async fn check(&self) -> BridgeResult<HealthCheckResult> {
        let running = self.running.read().await;
        let status = if *running {
            crate::health::types::HealthStatus::Healthy
        } else {
            crate::health::types::HealthStatus::Unhealthy
        };

        Ok(HealthCheckResult::new(
            "transform_processor".to_string(),
            status,
            if *running {
                "Transform processor is running".to_string()
            } else {
                "Transform processor is not running".to_string()
            },
        ))
    }

    fn component_name(&self) -> &str {
        "transform_processor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{logs::LogData, logs::LogLevel, TelemetryData, TelemetryType};

    #[tokio::test]
    async fn test_transform_processor_creation() {
        let config = TransformProcessorConfig::default();
        let processor = TransformProcessor::new(config);

        let stats = processor.get_stats().await.unwrap();
        assert_eq!(stats.total_records, 0);
    }

    #[tokio::test]
    async fn test_transform_processor_start_stop() {
        let config = TransformProcessorConfig::default();
        let processor = TransformProcessor::new(config);

        // Start the processor
        let result = processor.start().await;
        assert!(result.is_ok());

        // Check health
        let health = processor.health_check().await.unwrap();
        assert!(health);

        // Stop the processor
        let result = processor.stop().await;
        assert!(result.is_ok());

        // Check health again
        let health = processor.health_check().await.unwrap();
        assert!(!health);
    }

    #[tokio::test]
    async fn test_field_mapping_transformation() {
        let config = TransformProcessorConfig {
            transformations: vec![TransformOperation::FieldMapping {
                from: "old_field".to_string(),
                to: "new_field".to_string(),
            }],
            ..Default::default()
        };
        let processor = TransformProcessor::new(config);
        processor.start().await.unwrap();

        // Create test record
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp: chrono::Utc::now(),
                level: LogLevel::Info,
                message: "test message".to_string(),
                attributes: HashMap::new(),
                body: None,
                severity_text: Some("INFO".to_string()),
                severity_number: Some(9),
            }),
            attributes: HashMap::from([("old_field".to_string(), "test_value".to_string())]),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        let batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: "test".to_string(),
            size: 1,
            records: vec![record],
            metadata: HashMap::new(),
        };

        // Process batch
        let result = processor.process(batch).await;
        assert!(result.is_ok());

        let processed_batch = result.unwrap();
        assert_eq!(processed_batch.records.len(), 1);
        assert_eq!(processed_batch.records[0].status, ProcessingStatus::Success);

        // Check that field was mapped
        let new_field_value = processed_batch.records[0].metadata.get("attr_new_field");
        assert_eq!(new_field_value, Some(&"test_value".to_string()));

        processor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_add_attribute_transformation() {
        let config = TransformProcessorConfig {
            transformations: vec![TransformOperation::AddAttribute {
                key: "env".to_string(),
                value: "production".to_string(),
            }],
            ..Default::default()
        };
        let processor = TransformProcessor::new(config);
        processor.start().await.unwrap();

        // Create test record
        let record = TelemetryRecord {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            record_type: TelemetryType::Log,
            data: TelemetryData::Log(LogData {
                timestamp: chrono::Utc::now(),
                level: LogLevel::Info,
                message: "test message".to_string(),
                attributes: HashMap::new(),
                body: None,
                severity_text: Some("INFO".to_string()),
                severity_number: Some(9),
            }),
            attributes: HashMap::new(),
            tags: HashMap::new(),
            resource: None,
            service: None,
        };

        let batch = TelemetryBatch {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: "test".to_string(),
            size: 1,
            records: vec![record],
            metadata: HashMap::new(),
        };

        // Process batch
        let result = processor.process(batch).await;
        assert!(result.is_ok());

        let processed_batch = result.unwrap();
        assert_eq!(processed_batch.records.len(), 1);
        assert_eq!(processed_batch.records[0].status, ProcessingStatus::Success);

        // Check that attribute was added
        let env_value = processed_batch.records[0].metadata.get("attr_env");
        assert_eq!(env_value, Some(&"production".to_string()));

        processor.stop().await.unwrap();
    }
}
