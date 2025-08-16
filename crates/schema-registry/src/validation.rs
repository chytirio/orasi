//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Schema validation for the Schema Registry
//!
//! This module provides validation functionality for schemas and telemetry data.

use crate::error::SchemaRegistryResult;
use crate::schema::{Schema, SchemaFormat};
use async_trait::async_trait;
use bridge_core::types::TelemetryBatch;
use std::collections::HashMap;

/// Schema validator
pub struct SchemaValidator {
    /// Validation rules
    rules: HashMap<String, ValidationRule>,

    /// Strict validation mode
    strict_mode: bool,
}

impl SchemaValidator {
    /// Create a new schema validator
    pub fn new() -> Self {
        let mut validator = Self {
            rules: HashMap::new(),
            strict_mode: false,
        };

        // Add default validation rules
        validator.add_default_rules();

        validator
    }

    /// Create a validator with strict mode
    pub fn with_strict_mode(strict_mode: bool) -> Self {
        let mut validator = Self {
            rules: HashMap::new(),
            strict_mode,
        };

        validator.add_default_rules();

        validator
    }

    /// Add default validation rules
    fn add_default_rules(&mut self) {
        // Add schema format validation rules
        self.rules.insert(
            "schema_format".to_string(),
            ValidationRule {
                name: "schema_format".to_string(),
                description: "Validate schema format".to_string(),
                rule_type: ValidationRuleType::SchemaFormat,
                enabled: true,
            },
        );

        // Add schema content validation rules
        self.rules.insert(
            "schema_content".to_string(),
            ValidationRule {
                name: "schema_content".to_string(),
                description: "Validate schema content".to_string(),
                rule_type: ValidationRuleType::SchemaContent,
                enabled: true,
            },
        );

        // Add schema metadata validation rules
        self.rules.insert(
            "schema_metadata".to_string(),
            ValidationRule {
                name: "schema_metadata".to_string(),
                description: "Validate schema metadata".to_string(),
                rule_type: ValidationRuleType::SchemaMetadata,
                enabled: true,
            },
        );
    }

    /// Add custom validation rule
    pub fn add_rule(&mut self, rule: ValidationRule) {
        self.rules.insert(rule.name.clone(), rule);
    }

    /// Remove validation rule
    pub fn remove_rule(&mut self, rule_name: &str) {
        self.rules.remove(rule_name);
    }

    /// Enable or disable a rule
    pub fn set_rule_enabled(&mut self, rule_name: &str, enabled: bool) {
        if let Some(rule) = self.rules.get_mut(rule_name) {
            rule.enabled = enabled;
        }
    }
}

#[async_trait]
impl SchemaValidatorTrait for SchemaValidator {
    async fn validate_schema(&self, schema: &Schema) -> SchemaRegistryResult<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Run all enabled validation rules
        for rule in self.rules.values() {
            if !rule.enabled {
                continue;
            }

            match rule.rule_type {
                ValidationRuleType::SchemaFormat => {
                    self.validate_schema_format(schema, &mut errors, &mut warnings)?;
                }
                ValidationRuleType::SchemaContent => {
                    self.validate_schema_content(schema, &mut errors, &mut warnings)?;
                }
                ValidationRuleType::SchemaMetadata => {
                    self.validate_schema_metadata(schema, &mut errors, &mut warnings)?;
                }
                ValidationRuleType::Custom => {
                    // Custom rules would be implemented here
                }
            }
        }

        // Determine validation status
        let status = if errors.is_empty() {
            if warnings.is_empty() {
                ValidationStatus::Valid
            } else {
                ValidationStatus::Warning
            }
        } else {
            ValidationStatus::Invalid
        };

        Ok(ValidationResult {
            status,
            errors,
            warnings,
            metadata: HashMap::new(),
        })
    }

    async fn validate_data(
        &self,
        schema: &Schema,
        data: &TelemetryBatch,
    ) -> SchemaRegistryResult<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Basic data validation against schema
        self.validate_data_against_schema(schema, data, &mut errors, &mut warnings)?;

        // Determine validation status
        let status = if errors.is_empty() {
            if warnings.is_empty() {
                ValidationStatus::Valid
            } else {
                ValidationStatus::Warning
            }
        } else {
            ValidationStatus::Invalid
        };

        Ok(ValidationResult {
            status,
            errors,
            warnings,
            metadata: HashMap::new(),
        })
    }
}

impl SchemaValidator {
    /// Validate schema format
    fn validate_schema_format(
        &self,
        schema: &Schema,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> SchemaRegistryResult<()> {
        // Check if schema format is supported
        match schema.format {
            SchemaFormat::Json => {
                // Validate JSON format
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&schema.content) {
                    errors.push(ValidationError {
                        code: "INVALID_JSON_FORMAT".to_string(),
                        message: format!("Invalid JSON format: {}", e),
                        location: Some("content".to_string()),
                        details: None,
                    });
                }
            }
            SchemaFormat::Yaml => {
                // Validate YAML format
                if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(&schema.content) {
                    errors.push(ValidationError {
                        code: "INVALID_YAML_FORMAT".to_string(),
                        message: format!("Invalid YAML format: {}", e),
                        location: Some("content".to_string()),
                        details: None,
                    });
                }
            }
            SchemaFormat::Avro => {
                // Basic Avro validation
                if !schema.content.contains("\"type\"") {
                    errors.push(ValidationError {
                        code: "INVALID_AVRO_FORMAT".to_string(),
                        message: "Avro schema must contain 'type' field".to_string(),
                        location: Some("content".to_string()),
                        details: None,
                    });
                }
            }
            SchemaFormat::Protobuf => {
                // Basic Protobuf validation
                if !schema.content.contains("message") && !schema.content.contains("enum") {
                    errors.push(ValidationError {
                        code: "INVALID_PROTOBUF_FORMAT".to_string(),
                        message: "Protobuf schema must contain message or enum definitions"
                            .to_string(),
                        location: Some("content".to_string()),
                        details: None,
                    });
                }
            }
            SchemaFormat::OpenApi => {
                // Basic OpenAPI validation
                if !schema.content.contains("openapi") {
                    errors.push(ValidationError {
                        code: "INVALID_OPENAPI_FORMAT".to_string(),
                        message: "OpenAPI schema must contain 'openapi' field".to_string(),
                        location: Some("content".to_string()),
                        details: None,
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate schema content
    fn validate_schema_content(
        &self,
        schema: &Schema,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> SchemaRegistryResult<()> {
        // Check schema size
        if schema.content.is_empty() {
            errors.push(ValidationError {
                code: "EMPTY_SCHEMA_CONTENT".to_string(),
                message: "Schema content cannot be empty".to_string(),
                location: Some("content".to_string()),
                details: None,
            });
        }

        // Check schema size limits
        let max_size = 1024 * 1024; // 1MB
        if schema.content.len() > max_size {
            errors.push(ValidationError {
                code: "SCHEMA_TOO_LARGE".to_string(),
                message: format!("Schema content exceeds maximum size of {} bytes", max_size),
                location: Some("content".to_string()),
                details: None,
            });
        }

        // Check for suspicious content
        if schema.content.contains("script") || schema.content.contains("javascript") {
            warnings.push(ValidationWarning {
                code: "SUSPICIOUS_CONTENT".to_string(),
                message: "Schema content contains potentially suspicious patterns".to_string(),
                location: Some("content".to_string()),
                details: None,
            });
        }

        Ok(())
    }

    /// Validate schema metadata
    fn validate_schema_metadata(
        &self,
        schema: &Schema,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> SchemaRegistryResult<()> {
        // Check required fields
        if schema.name.is_empty() {
            errors.push(ValidationError {
                code: "MISSING_SCHEMA_NAME".to_string(),
                message: "Schema name is required".to_string(),
                location: Some("name".to_string()),
                details: None,
            });
        }

        // Check name format
        if !schema
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            warnings.push(ValidationWarning {
                code: "INVALID_SCHEMA_NAME".to_string(),
                message: "Schema name contains non-alphanumeric characters".to_string(),
                location: Some("name".to_string()),
                details: None,
            });
        }

        // Check description length
        if let Some(description) = &schema.description {
            if description.len() > 1000 {
                warnings.push(ValidationWarning {
                    code: "DESCRIPTION_TOO_LONG".to_string(),
                    message: "Schema description is very long".to_string(),
                    location: Some("description".to_string()),
                    details: None,
                });
            }
        }

        // Check tags
        if schema.tags.len() > 20 {
            warnings.push(ValidationWarning {
                code: "TOO_MANY_TAGS".to_string(),
                message: "Schema has many tags".to_string(),
                location: Some("tags".to_string()),
                details: None,
            });
        }

        Ok(())
    }

    /// Validate data against schema
    fn validate_data_against_schema(
        &self,
        schema: &Schema,
        data: &TelemetryBatch,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> SchemaRegistryResult<()> {
        // Basic validation - check if data structure matches schema expectations
        // This is a simplified implementation

        // Check if data has the expected number of records
        if data.records.is_empty() {
            warnings.push(ValidationWarning {
                code: "EMPTY_DATA".to_string(),
                message: "Telemetry batch contains no records".to_string(),
                location: None,
                details: None,
            });
        }

        // Check data size
        let data_size = data.records.len();
        if data_size > 10000 {
            warnings.push(ValidationWarning {
                code: "LARGE_DATA_BATCH".to_string(),
                message: format!("Telemetry batch contains {} records", data_size),
                location: None,
                details: None,
            });
        }

        // Validate individual records
        for (i, record) in data.records.iter().enumerate() {
            self.validate_record_against_schema(schema, record, i, errors, warnings)?;
        }

        Ok(())
    }

    /// Validate individual record against schema
    fn validate_record_against_schema(
        &self,
        schema: &Schema,
        record: &bridge_core::types::TelemetryRecord,
        index: usize,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> SchemaRegistryResult<()> {
        // Check record type against schema type
        let record_type_matches = match schema.schema_type {
            crate::schema::SchemaType::Metric => matches!(
                record.record_type,
                bridge_core::types::TelemetryType::Metric
            ),
            crate::schema::SchemaType::Trace => {
                matches!(record.record_type, bridge_core::types::TelemetryType::Trace)
            }
            crate::schema::SchemaType::Log => {
                matches!(record.record_type, bridge_core::types::TelemetryType::Log)
            }
            crate::schema::SchemaType::Event => {
                matches!(record.record_type, bridge_core::types::TelemetryType::Event)
            }
            _ => true, // Custom types are always considered valid
        };

        if !record_type_matches {
            errors.push(ValidationError {
                code: "RECORD_TYPE_MISMATCH".to_string(),
                message: format!("Record type does not match schema type at index {}", index),
                location: Some(format!("records[{}].record_type", index)),
                details: None,
            });
        }

        // Check required attributes based on schema
        if let Some(resource) = &record.resource {
            if resource.resource_type.is_empty() {
                warnings.push(ValidationWarning {
                    code: "MISSING_SERVICE_NAME".to_string(),
                    message: format!("Record at index {} has empty service name", index),
                    location: Some(format!("records[{}].resource.service_name", index)),
                    details: None,
                });
            }
        }

        Ok(())
    }
}

/// Schema validator trait
#[async_trait]
pub trait SchemaValidatorTrait: Send + Sync {
    /// Validate a schema
    async fn validate_schema(&self, schema: &Schema) -> SchemaRegistryResult<ValidationResult>;

    /// Validate telemetry data against a schema
    async fn validate_data(
        &self,
        schema: &Schema,
        data: &TelemetryBatch,
    ) -> SchemaRegistryResult<ValidationResult>;
}

/// Validation result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationResult {
    /// Validation status
    pub status: ValidationStatus,

    /// Validation errors
    pub errors: Vec<ValidationError>,

    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ValidationResult {
    /// Check if validation is successful
    pub fn is_valid(&self) -> bool {
        matches!(
            self.status,
            ValidationStatus::Valid | ValidationStatus::Warning
        )
    }

    /// Check if validation has errors
    pub fn has_errors(&self) -> bool {
        matches!(self.status, ValidationStatus::Invalid)
    }

    /// Check if validation has warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Get warning count
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }
}

/// Validation status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum ValidationStatus {
    /// Validation passed
    Valid,

    /// Validation passed with warnings
    Warning,

    /// Validation failed
    Invalid,
}

/// Validation error
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Error location (optional)
    pub location: Option<String>,

    /// Additional details (optional)
    pub details: Option<String>,
}

/// Validation warning
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,

    /// Warning message
    pub message: String,

    /// Warning location (optional)
    pub location: Option<String>,

    /// Additional details (optional)
    pub details: Option<String>,
}

/// Validation rule
#[derive(Debug, Clone)]
pub struct ValidationRule {
    /// Rule name
    pub name: String,

    /// Rule description
    pub description: String,

    /// Rule type
    pub rule_type: ValidationRuleType,

    /// Whether the rule is enabled
    pub enabled: bool,
}

/// Validation rule type
#[derive(Debug, Clone)]
pub enum ValidationRuleType {
    /// Schema format validation
    SchemaFormat,

    /// Schema content validation
    SchemaContent,

    /// Schema metadata validation
    SchemaMetadata,

    /// Custom validation rule
    Custom,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Schema, SchemaFormat, SchemaType, SchemaVersion};

    #[tokio::test]
    async fn test_schema_validator_creation() {
        let validator = SchemaValidator::new();
        assert!(!validator.strict_mode);
    }

    #[tokio::test]
    async fn test_schema_validation() {
        let validator = SchemaValidator::new();

        let schema = Schema::new(
            "test-schema".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object", "properties": {"value": {"type": "number"}}}"#.to_string(),
            SchemaFormat::Json,
        );

        let result = validator.validate_schema(&schema).await.unwrap();
        assert!(result.is_valid());
        assert_eq!(result.error_count(), 0);
    }

    #[tokio::test]
    async fn test_invalid_schema_validation() {
        let validator = SchemaValidator::new();

        let schema = Schema::new(
            "".to_string(), // Empty name
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"invalid": json}"#.to_string(), // Invalid JSON
            SchemaFormat::Json,
        );

        let result = validator.validate_schema(&schema).await.unwrap();
        assert!(!result.is_valid());
        assert!(result.error_count() > 0);
    }

    #[tokio::test]
    async fn test_data_validation() {
        let validator = SchemaValidator::new();

        let schema = Schema::new(
            "test-schema".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object"}"#.to_string(),
            SchemaFormat::Json,
        );

        let data = TelemetryBatch {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: "test".to_string(),
            size: 1,
            records: vec![bridge_core::types::TelemetryRecord {
                id: uuid::Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
                record_type: bridge_core::types::TelemetryType::Metric,
                data: bridge_core::types::TelemetryData::Metric(bridge_core::types::MetricData {
                    name: "test_metric".to_string(),
                    description: None,
                    unit: None,
                    metric_type: bridge_core::types::MetricType::Gauge,
                    value: bridge_core::types::MetricValue::Gauge(42.0),
                    labels: HashMap::new(),
                    timestamp: chrono::Utc::now(),
                }),
                attributes: HashMap::new(),
                tags: HashMap::new(),
                resource: None,
                service: None,
            }],
            metadata: HashMap::new(),
        };

        let result = validator.validate_data(&schema, &data).await.unwrap();
        assert!(result.is_valid());
    }
}
