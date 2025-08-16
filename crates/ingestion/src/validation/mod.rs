//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Telemetry data validation utilities
//!
//! This module provides utilities for validating telemetry data
//! and ensuring data quality.

use bridge_core::types::{TelemetryData, TelemetryRecord, TelemetryType};
use bridge_core::{BridgeResult, TelemetryBatch};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validation trait for telemetry data
#[async_trait::async_trait]
pub trait TelemetryValidator: Send + Sync {
    /// Validate telemetry data
    async fn validate(&self, data: &TelemetryBatch) -> BridgeResult<ValidationResult>;

    /// Get validation rules
    fn get_validation_rules(&self) -> Vec<ValidationRule>;

    /// Add validation rule
    fn add_validation_rule(&mut self, rule: ValidationRule);

    /// Remove validation rule
    fn remove_validation_rule(&mut self, rule_name: &str);
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Is valid
    pub is_valid: bool,

    /// Validation errors
    pub errors: Vec<ValidationError>,

    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,

    /// Validation timestamp
    pub timestamp: DateTime<Utc>,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Error severity
    pub severity: ValidationSeverity,

    /// Field path
    pub field_path: Option<String>,

    /// Record index
    pub record_index: Option<usize>,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,

    /// Warning message
    pub message: String,

    /// Warning severity
    pub severity: ValidationSeverity,

    /// Field path
    pub field_path: Option<String>,

    /// Record index
    pub record_index: Option<usize>,
}

/// Validation severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// Rule name
    pub name: String,

    /// Rule type
    pub rule_type: ValidationRuleType,

    /// Rule enabled
    pub enabled: bool,

    /// Rule parameters
    pub parameters: HashMap<String, String>,
}

/// Validation rule type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRuleType {
    RequiredField,
    FieldType,
    FieldRange,
    FieldFormat,
    Custom,
}

/// Schema validator implementation
pub struct SchemaValidator {
    rules: Vec<ValidationRule>,
}

impl SchemaValidator {
    /// Create new schema validator
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Validate schema
    async fn validate_schema(&self, batch: &TelemetryBatch) -> BridgeResult<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate batch structure
        if batch.id.to_string().is_empty() {
            errors.push(ValidationError {
                code: "EMPTY_BATCH_ID".to_string(),
                message: "Batch ID cannot be empty".to_string(),
                severity: ValidationSeverity::High,
                field_path: Some("id".to_string()),
                record_index: None,
            });
        }

        if batch.source.is_empty() {
            errors.push(ValidationError {
                code: "EMPTY_SOURCE".to_string(),
                message: "Source cannot be empty".to_string(),
                severity: ValidationSeverity::High,
                field_path: Some("source".to_string()),
                record_index: None,
            });
        }

        // Validate records
        for (index, record) in batch.records.iter().enumerate() {
            let (record_errors, record_warnings) = self.validate_record(record, index);
            errors.extend(record_errors);
            warnings.extend(record_warnings);
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            timestamp: Utc::now(),
        })
    }

    /// Validate individual record
    fn validate_record(
        &self,
        record: &TelemetryRecord,
        index: usize,
    ) -> (Vec<ValidationError>, Vec<ValidationWarning>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate record ID
        if record.id.to_string().is_empty() {
            errors.push(ValidationError {
                code: "EMPTY_RECORD_ID".to_string(),
                message: "Record ID cannot be empty".to_string(),
                severity: ValidationSeverity::High,
                field_path: Some("id".to_string()),
                record_index: Some(index),
            });
        }

        // Validate record type
        match &record.record_type {
            TelemetryType::Metric => {
                if let TelemetryData::Metric(metric) = &record.data {
                    if metric.name.is_empty() {
                        errors.push(ValidationError {
                            code: "EMPTY_METRIC_NAME".to_string(),
                            message: "Metric name cannot be empty".to_string(),
                            severity: ValidationSeverity::High,
                            field_path: Some("data.metric.name".to_string()),
                            record_index: Some(index),
                        });
                    }
                }
            }
            TelemetryType::Trace => {
                if let TelemetryData::Trace(trace) = &record.data {
                    if trace.trace_id.is_empty() {
                        errors.push(ValidationError {
                            code: "EMPTY_TRACE_ID".to_string(),
                            message: "Trace ID cannot be empty".to_string(),
                            severity: ValidationSeverity::High,
                            field_path: Some("data.trace.trace_id".to_string()),
                            record_index: Some(index),
                        });
                    }

                    if trace.span_id.is_empty() {
                        errors.push(ValidationError {
                            code: "EMPTY_SPAN_ID".to_string(),
                            message: "Span ID cannot be empty".to_string(),
                            severity: ValidationSeverity::High,
                            field_path: Some("data.trace.span_id".to_string()),
                            record_index: Some(index),
                        });
                    }
                }
            }
            TelemetryType::Log => {
                if let TelemetryData::Log(log) = &record.data {
                    if log.message.is_empty() {
                        errors.push(ValidationError {
                            code: "EMPTY_LOG_MESSAGE".to_string(),
                            message: "Log message cannot be empty".to_string(),
                            severity: ValidationSeverity::High,
                            field_path: Some("data.log.message".to_string()),
                            record_index: Some(index),
                        });
                    }
                }
            }
            TelemetryType::Event => {
                // Add event validation
                if let TelemetryData::Event(event_data) = &record.data {
                    // Validate event name
                    if event_data.name.trim().is_empty() {
                        errors.push(ValidationError {
                            code: "EMPTY_EVENT_NAME".to_string(),
                            message: "Event name cannot be empty".to_string(),
                            severity: ValidationSeverity::High,
                            field_path: Some("data.event.name".to_string()),
                            record_index: Some(index),
                        });
                    }

                    // Validate event timestamp
                    if event_data.timestamp > Utc::now() {
                        errors.push(ValidationError {
                            code: "FUTURE_EVENT_TIMESTAMP".to_string(),
                            message: "Event timestamp cannot be in the future".to_string(),
                            severity: ValidationSeverity::Medium,
                            field_path: Some("data.event.timestamp".to_string()),
                            record_index: Some(index),
                        });
                    }

                    // Validate event attributes (optional)
                    if event_data.attributes.is_empty() {
                        warnings.push(ValidationWarning {
                            code: "EMPTY_EVENT_ATTRIBUTES".to_string(),
                            message: "Event has no attributes".to_string(),
                            severity: ValidationSeverity::Low,
                            field_path: Some("data.event.attributes".to_string()),
                            record_index: Some(index),
                        });
                    }
                }
            }
        }

        (errors, warnings)
    }
}

#[async_trait::async_trait]
impl TelemetryValidator for SchemaValidator {
    async fn validate(&self, data: &TelemetryBatch) -> BridgeResult<ValidationResult> {
        self.validate_schema(data).await
    }

    fn get_validation_rules(&self) -> Vec<ValidationRule> {
        self.rules.clone()
    }

    fn add_validation_rule(&mut self, rule: ValidationRule) {
        self.rules.push(rule);
    }

    fn remove_validation_rule(&mut self, rule_name: &str) {
        self.rules.retain(|rule| rule.name != rule_name);
    }
}

/// Data validator implementation
pub struct DataValidator {
    rules: Vec<ValidationRule>,
}

impl DataValidator {
    /// Create new data validator
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Validate data quality
    async fn validate_data_quality(
        &self,
        batch: &TelemetryBatch,
    ) -> BridgeResult<ValidationResult> {
        let errors = Vec::new();
        let mut warnings = Vec::new();

        // Check for duplicate records
        let mut seen_ids = std::collections::HashSet::new();
        for (index, record) in batch.records.iter().enumerate() {
            if !seen_ids.insert(record.id) {
                warnings.push(ValidationWarning {
                    code: "DUPLICATE_RECORD_ID".to_string(),
                    message: "Duplicate record ID found".to_string(),
                    severity: ValidationSeverity::Medium,
                    field_path: Some("id".to_string()),
                    record_index: Some(index),
                });
            }
        }

        // Check timestamp consistency
        let now = Utc::now();
        for (index, record) in batch.records.iter().enumerate() {
            let time_diff = now.signed_duration_since(record.timestamp);
            if time_diff.num_hours() > 24 {
                warnings.push(ValidationWarning {
                    code: "OLD_TIMESTAMP".to_string(),
                    message: "Record timestamp is more than 24 hours old".to_string(),
                    severity: ValidationSeverity::Low,
                    field_path: Some("timestamp".to_string()),
                    record_index: Some(index),
                });
            }
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            timestamp: Utc::now(),
        })
    }
}

#[async_trait::async_trait]
impl TelemetryValidator for DataValidator {
    async fn validate(&self, data: &TelemetryBatch) -> BridgeResult<ValidationResult> {
        self.validate_data_quality(data).await
    }

    fn get_validation_rules(&self) -> Vec<ValidationRule> {
        self.rules.clone()
    }

    fn add_validation_rule(&mut self, rule: ValidationRule) {
        self.rules.push(rule);
    }

    fn remove_validation_rule(&mut self, rule_name: &str) {
        self.rules.retain(|rule| rule.name != rule_name);
    }
}

/// Validation factory
pub struct ValidationFactory;

impl ValidationFactory {
    /// Create validator for specific type
    pub fn create_validator(validator_type: &str) -> Option<Box<dyn TelemetryValidator>> {
        match validator_type {
            "schema" => Some(Box::new(SchemaValidator::new())),
            "data" => Some(Box::new(DataValidator::new())),
            _ => None,
        }
    }

    /// Get supported validator types
    pub fn get_supported_validator_types() -> Vec<String> {
        vec!["schema".to_string(), "data".to_string()]
    }
}
