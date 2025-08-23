//! API request structures
//!
//! This module contains all the request structures for the API endpoints.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::schema::{Schema, SchemaFormat, SchemaType, SchemaVersion};

/// Register schema request
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterSchemaRequest {
    /// Schema name
    pub name: String,

    /// Schema version
    pub version: SchemaVersion,

    /// Schema type
    pub schema_type: SchemaType,

    /// Schema content
    pub content: String,

    /// Schema format
    pub format: SchemaFormat,
}

impl RegisterSchemaRequest {
    /// Validate the request
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate name
        if self.name.trim().is_empty() {
            errors.push("Schema name cannot be empty".to_string());
        } else if self.name.len() > 255 {
            errors.push("Schema name cannot exceed 255 characters".to_string());
        } else if !self
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            errors.push(
                "Schema name can only contain alphanumeric characters, underscores, and hyphens"
                    .to_string(),
            );
        }

        // Validate version
        if self.version.major == 0 && self.version.minor == 0 && self.version.patch == 0 {
            errors.push("Schema version cannot be 0.0.0".to_string());
        }

        // Validate content
        if self.content.trim().is_empty() {
            errors.push("Schema content cannot be empty".to_string());
        } else if self.content.len() > 1024 * 1024 {
            // 1MB limit
            errors.push("Schema content cannot exceed 1MB".to_string());
        }

        // Validate format-specific content
        match self.format {
            SchemaFormat::Json => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&self.content) {
                    errors.push(format!("Invalid JSON content: {}", e));
                }
            }
            SchemaFormat::Yaml => {
                if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(&self.content) {
                    errors.push(format!("Invalid YAML content: {}", e));
                }
            }
            SchemaFormat::Protobuf => {
                // Basic protobuf validation - check for message syntax
                if !self.content.contains("message") && !self.content.contains("syntax") {
                    errors.push("Protobuf content should contain message definitions".to_string());
                }
            }
            SchemaFormat::Avro => {
                // Basic Avro validation - check for JSON structure
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&self.content) {
                    errors.push(format!("Invalid Avro JSON content: {}", e));
                }
            }
            SchemaFormat::OpenApi => {
                // Basic OpenAPI validation - check for JSON structure
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&self.content) {
                    errors.push(format!("Invalid OpenAPI JSON content: {}", e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Validate data request
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateDataRequest {
    /// Telemetry data to validate
    pub data: bridge_core::types::TelemetryBatch,
}

impl ValidateDataRequest {
    /// Validate the request
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate that data is not empty
        if self.data.records.is_empty() {
            errors.push("Telemetry data cannot be empty".to_string());
        }

        // Validate individual records
        for (i, record) in self.data.records.iter().enumerate() {
            // Check if timestamp is reasonable (not too far in past/future)
            let now = chrono::Utc::now();
            let time_diff = (record.timestamp - now).num_seconds().abs();
            if time_diff > 86400 {
                // More than 24 hours difference
                errors.push(format!(
                    "Record {} has timestamp too far from current time",
                    i
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Validate schema request
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateSchemaRequest {
    /// Schema to validate
    pub schema: Schema,
}

impl ValidateSchemaRequest {
    /// Validate the request
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate schema name
        if self.schema.name.trim().is_empty() {
            errors.push("Schema name cannot be empty".to_string());
        }

        // Validate schema content
        if self.schema.content.trim().is_empty() {
            errors.push("Schema content cannot be empty".to_string());
        }

        // Validate schema format
        match self.schema.format {
            SchemaFormat::Json => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&self.schema.content) {
                    errors.push(format!("Invalid JSON schema: {}", e));
                }
            }
            SchemaFormat::Yaml => {
                if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(&self.schema.content) {
                    errors.push(format!("Invalid YAML schema: {}", e));
                }
            }
            SchemaFormat::Protobuf => {
                if !self.schema.content.contains("message") {
                    errors.push("Protobuf schema should contain message definitions".to_string());
                }
            }
            SchemaFormat::Avro => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&self.schema.content) {
                    errors.push(format!("Invalid Avro schema: {}", e));
                }
            }
            SchemaFormat::OpenApi => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&self.schema.content) {
                    errors.push(format!("Invalid OpenAPI schema: {}", e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Evolve schema request
#[derive(Debug, Serialize, Deserialize)]
pub struct EvolveSchemaRequest {
    /// Schema to evolve
    pub schema: Schema,
}

impl EvolveSchemaRequest {
    /// Validate the request
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate schema name
        if self.schema.name.trim().is_empty() {
            errors.push("Schema name cannot be empty".to_string());
        }

        // Validate schema content
        if self.schema.content.trim().is_empty() {
            errors.push("Schema content cannot be empty".to_string());
        }

        // Validate that this is a new version
        if self.schema.version.major == 0
            && self.schema.version.minor == 0
            && self.schema.version.patch == 0
        {
            errors.push("Schema version cannot be 0.0.0 for evolution".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{SchemaFormat, SchemaType, SchemaVersion};

    #[test]
    fn test_register_schema_request_validation() {
        // Valid request
        let valid_request = RegisterSchemaRequest {
            name: "test_schema".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            schema_type: SchemaType::Metric,
            content: r#"{"type": "object", "properties": {"test": {"type": "string"}}}"#
                .to_string(),
            format: SchemaFormat::Json,
        };
        assert!(valid_request.validate().is_ok());

        // Invalid name
        let invalid_name = RegisterSchemaRequest {
            name: "".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            schema_type: SchemaType::Metric,
            content: r#"{"type": "object"}"#.to_string(),
            format: SchemaFormat::Json,
        };
        assert!(invalid_name.validate().is_err());

        // Invalid JSON content
        let invalid_json = RegisterSchemaRequest {
            name: "test".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            schema_type: SchemaType::Metric,
            content: "{ invalid json }".to_string(),
            format: SchemaFormat::Json,
        };
        assert!(invalid_json.validate().is_err());
    }

    #[test]
    fn test_validate_schema_request_validation() {
        let schema = Schema::new(
            "test".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object", "properties": {"test": {"type": "string"}}}"#.to_string(),
            SchemaFormat::Json,
        );

        let request = ValidateSchemaRequest { schema };
        assert!(request.validate().is_ok());
    }
}
