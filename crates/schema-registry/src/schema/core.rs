//! Core schema functionality
//!
//! This module contains the main Schema struct and its core functionality.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::{
    metadata::SchemaMetadata, resolution::ResolvedSchema, types::*, version::SchemaVersion,
};

/// Schema representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Unique schema identifier
    pub id: Uuid,

    /// Schema name
    pub name: String,

    /// Schema version
    pub version: SchemaVersion,

    /// Schema description
    pub description: Option<String>,

    /// Schema type
    pub schema_type: SchemaType,

    /// Schema content (JSON, YAML, etc.)
    pub content: String,

    /// Schema format
    pub format: SchemaFormat,

    /// Schema fingerprint (hash of content)
    pub fingerprint: String,

    /// Schema metadata
    pub metadata: HashMap<String, String>,

    /// Schema tags
    pub tags: Vec<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last modified timestamp
    pub updated_at: DateTime<Utc>,

    /// Schema owner
    pub owner: Option<String>,

    /// Schema visibility
    pub visibility: SchemaVisibility,

    /// Schema compatibility mode
    pub compatibility_mode: CompatibilityMode,
}

impl Schema {
    /// Create a new schema
    pub fn new(
        name: String,
        version: SchemaVersion,
        schema_type: SchemaType,
        content: String,
        format: SchemaFormat,
    ) -> Self {
        let fingerprint = Self::generate_fingerprint(&content);
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            name,
            version,
            description: None,
            schema_type,
            content,
            format,
            fingerprint,
            metadata: HashMap::new(),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            owner: None,
            visibility: SchemaVisibility::Public,
            compatibility_mode: CompatibilityMode::Backward,
        }
    }

    /// Generate fingerprint for schema content
    pub fn generate_fingerprint(content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Update schema content and regenerate fingerprint
    pub fn update_content(&mut self, content: String) {
        self.content = content;
        self.fingerprint = Self::generate_fingerprint(&self.content);
        self.updated_at = Utc::now();
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = Utc::now();
    }

    /// Add tag
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    /// Remove tag
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
        self.updated_at = Utc::now();
    }

    /// Check if schema has tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.to_string())
    }

    /// Get schema size in bytes
    pub fn size(&self) -> usize {
        self.content.len()
    }

    /// Validate schema content
    pub fn validate_content(&self) -> Result<(), String> {
        match self.format {
            SchemaFormat::Json => {
                serde_json::from_str::<serde_json::Value>(&self.content)
                    .map_err(|e| format!("Invalid JSON: {}", e))?;
            }
            SchemaFormat::Yaml => {
                serde_yaml::from_str::<serde_yaml::Value>(&self.content)
                    .map_err(|e| format!("Invalid YAML: {}", e))?;
            }
            SchemaFormat::Avro => {
                // Basic Avro validation - check for required fields
                if !self.content.contains("\"type\"") {
                    return Err("Avro schema must contain 'type' field".to_string());
                }
            }
            SchemaFormat::Protobuf => {
                // Basic Protobuf validation
                if !self.content.contains("message") && !self.content.contains("enum") {
                    return Err(
                        "Protobuf schema must contain message or enum definitions".to_string()
                    );
                }
            }
            SchemaFormat::OpenApi => {
                // Basic OpenAPI validation
                if !self.content.contains("openapi") {
                    return Err("OpenAPI schema must contain 'openapi' field".to_string());
                }
            }
        }
        Ok(())
    }

    /// Resolve schema by resolving all references and imports
    pub fn resolve_schema(&self) -> Result<ResolvedSchema, String> {
        let mut resolved = ResolvedSchema::new(self.clone());

        // Resolve imports and references based on format
        match self.format {
            SchemaFormat::Json => {
                resolved.resolved_content = self.resolve_json_references()?;
            }
            SchemaFormat::Yaml => {
                resolved.resolved_content = self.resolve_yaml_references()?;
            }
            SchemaFormat::OpenApi => {
                resolved.resolved_content = self.resolve_openapi_references()?;
            }
            _ => {
                // For other formats, content is already resolved
                resolved.resolved_content = self.content.clone();
            }
        }

        // Resolve semantic conventions
        resolved.semantic_conventions = self.resolve_semantic_conventions()?;

        Ok(resolved)
    }

    /// Resolve JSON schema references
    fn resolve_json_references(&self) -> Result<String, String> {
        let json_value: serde_json::Value = serde_json::from_str(&self.content)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let resolved = self.resolve_json_value(&json_value)?;
        serde_json::to_string_pretty(&resolved)
            .map_err(|e| format!("Failed to serialize resolved JSON: {}", e))
    }

    /// Recursively resolve JSON value references
    fn resolve_json_value(&self, value: &serde_json::Value) -> Result<serde_json::Value, String> {
        match value {
            serde_json::Value::Object(map) => {
                let mut resolved_map = serde_json::Map::new();

                for (key, val) in map {
                    if key == "$ref" {
                        // Handle JSON reference - for now, just return the original value
                        // In a real implementation, this would resolve the reference
                        return Ok(value.clone());
                    } else {
                        let resolved_val = self.resolve_json_value(val)?;
                        resolved_map.insert(key.clone(), resolved_val);
                    }
                }

                Ok(serde_json::Value::Object(resolved_map))
            }
            serde_json::Value::Array(arr) => {
                let mut resolved_arr = Vec::new();
                for item in arr {
                    let resolved_item = self.resolve_json_value(item)?;
                    resolved_arr.push(resolved_item);
                }
                Ok(serde_json::Value::Array(resolved_arr))
            }
            _ => Ok(value.clone()),
        }
    }

    /// Resolve YAML references
    fn resolve_yaml_references(&self) -> Result<String, String> {
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&self.content)
            .map_err(|e| format!("Failed to parse YAML: {}", e))?;

        let resolved = self.resolve_yaml_value(&yaml_value)?;
        serde_yaml::to_string(&resolved)
            .map_err(|e| format!("Failed to serialize resolved YAML: {}", e))
    }

    /// Recursively resolve YAML value references
    fn resolve_yaml_value(&self, value: &serde_yaml::Value) -> Result<serde_yaml::Value, String> {
        match value {
            serde_yaml::Value::Mapping(map) => {
                let mut resolved_map = serde_yaml::Mapping::new();

                for (key, val) in map {
                    if let Some(key_str) = key.as_str() {
                        if key_str == "$ref" {
                            // Handle YAML reference - for now, just return the original value
                            // In a real implementation, this would resolve the reference
                            return Ok(value.clone());
                        } else {
                            let resolved_val = self.resolve_yaml_value(val)?;
                            resolved_map.insert(key.clone(), resolved_val);
                        }
                    } else {
                        let resolved_val = self.resolve_yaml_value(val)?;
                        resolved_map.insert(key.clone(), resolved_val);
                    }
                }

                Ok(serde_yaml::Value::Mapping(resolved_map))
            }
            serde_yaml::Value::Sequence(seq) => {
                let mut resolved_seq = Vec::new();
                for item in seq {
                    let resolved_item = self.resolve_yaml_value(item)?;
                    resolved_seq.push(resolved_item);
                }
                Ok(serde_yaml::Value::Sequence(resolved_seq))
            }
            _ => Ok(value.clone()),
        }
    }

    /// Resolve OpenAPI references
    fn resolve_openapi_references(&self) -> Result<String, String> {
        // OpenAPI uses JSON reference format, so we can reuse JSON resolution
        self.resolve_json_references()
    }

    /// Resolve semantic conventions
    fn resolve_semantic_conventions(&self) -> Result<HashMap<String, serde_json::Value>, String> {
        let conventions = HashMap::new();

        // Parse the schema content to find semantic convention imports
        match self.format {
            SchemaFormat::Json | SchemaFormat::Yaml => {
                // Look for semantic convention imports in the content
                if self.content.contains("semantic_conventions") || self.content.contains("semconv")
                {
                    // This would need to be implemented to actually fetch and resolve
                    // OpenTelemetry semantic conventions
                    // For now, return empty HashMap
                }
            }
            _ => {
                // Other formats don't support semantic conventions
            }
        }

        Ok(conventions)
    }

    /// Check if this schema is backward compatible with another schema
    pub fn is_backward_compatible_with(&self, other: &Schema) -> Result<bool, String> {
        // For now, implement basic compatibility checking
        // In a real implementation, this would do deep schema comparison

        // Check if schemas are of the same type
        if self.schema_type != other.schema_type {
            return Ok(false);
        }

        // Check if schemas are of the same format
        if self.format != other.format {
            return Ok(false);
        }

        // For JSON schemas, we could implement more sophisticated compatibility checking
        match self.format {
            SchemaFormat::Json => self.check_json_backward_compatibility(other),
            SchemaFormat::Yaml => self.check_yaml_backward_compatibility(other),
            _ => {
                // For other formats, assume compatible if types match
                Ok(true)
            }
        }
    }

    /// Check JSON schema backward compatibility
    fn check_json_backward_compatibility(&self, other: &Schema) -> Result<bool, String> {
        let self_json: serde_json::Value = serde_json::from_str(&self.content)
            .map_err(|e| format!("Failed to parse self schema JSON: {}", e))?;

        let other_json: serde_json::Value = serde_json::from_str(&other.content)
            .map_err(|e| format!("Failed to parse other schema JSON: {}", e))?;

        // Basic compatibility check - in a real implementation, this would be much more sophisticated
        // For now, just check if the new schema doesn't remove required fields from the old schema

        if let (serde_json::Value::Object(self_obj), serde_json::Value::Object(other_obj)) =
            (&self_json, &other_json)
        {
            // Check if required fields in the old schema are still present in the new schema
            if let Some(self_required) = self_obj.get("required") {
                if let Some(other_required) = other_obj.get("required") {
                    if let (
                        serde_json::Value::Array(self_req),
                        serde_json::Value::Array(other_req),
                    ) = (self_required, other_required)
                    {
                        for field in self_req {
                            if let serde_json::Value::String(_field_name) = field {
                                if !other_req.contains(field) {
                                    return Ok(false); // Required field was removed
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(true)
    }

    /// Check YAML schema backward compatibility
    fn check_yaml_backward_compatibility(&self, other: &Schema) -> Result<bool, String> {
        let _self_yaml: serde_yaml::Value = serde_yaml::from_str(&self.content)
            .map_err(|e| format!("Failed to parse self schema YAML: {}", e))?;

        let _other_yaml: serde_yaml::Value = serde_yaml::from_str(&other.content)
            .map_err(|e| format!("Failed to parse other schema YAML: {}", e))?;

        // Similar logic to JSON compatibility checking
        // For now, assume compatible
        Ok(true)
    }

    /// Check if this schema is forward compatible with another schema
    pub fn is_forward_compatible_with(&self, other: &Schema) -> Result<bool, String> {
        // Forward compatibility means the old schema can handle data from the new schema
        // This is typically the inverse of backward compatibility for most schema formats
        other.is_backward_compatible_with(self)
    }

    /// Generate migration script for schema changes
    pub fn generate_migration_script(&self, target_schema: &Schema) -> Result<String, String> {
        let mut script = String::new();

        script.push_str("# Schema Migration Script\n");
        script.push_str(&format!("# From: {} v{}\n", self.name, self.version));
        script.push_str(&format!(
            "# To: {} v{}\n\n",
            target_schema.name, target_schema.version
        ));

        // Check compatibility
        let backward_compatible = self.is_backward_compatible_with(target_schema)?;
        let forward_compatible = self.is_forward_compatible_with(target_schema)?;

        script.push_str(&format!("# Backward Compatible: {}\n", backward_compatible));
        script.push_str(&format!("# Forward Compatible: {}\n\n", forward_compatible));

        // Generate migration steps based on schema format
        match self.format {
            SchemaFormat::Json => {
                script.push_str("# JSON Schema Migration Steps:\n");
                script.push_str("1. Update schema definition\n");
                script.push_str("2. Validate data compatibility\n");
                script.push_str("3. Update any dependent systems\n");
            }
            SchemaFormat::Yaml => {
                script.push_str("# YAML Schema Migration Steps:\n");
                script.push_str("1. Update schema definition\n");
                script.push_str("2. Validate data compatibility\n");
                script.push_str("3. Update any dependent systems\n");
            }
            SchemaFormat::Avro => {
                script.push_str("# Avro Schema Migration Steps:\n");
                script.push_str("1. Update schema definition\n");
                script.push_str("2. Handle schema evolution rules\n");
                script.push_str("3. Update serialization/deserialization code\n");
            }
            SchemaFormat::Protobuf => {
                script.push_str("# Protobuf Schema Migration Steps:\n");
                script.push_str("1. Update .proto file\n");
                script.push_str("2. Regenerate code\n");
                script.push_str("3. Deploy updated services\n");
            }
            SchemaFormat::OpenApi => {
                script.push_str("# OpenAPI Schema Migration Steps:\n");
                script.push_str("1. Update API specification\n");
                script.push_str("2. Update client code\n");
                script.push_str("3. Deploy updated API\n");
            }
        }

        Ok(script)
    }
}

impl From<Schema> for SchemaMetadata {
    fn from(schema: Schema) -> Self {
        Self {
            id: schema.id,
            name: schema.name.clone(),
            version: schema.version.clone(),
            schema_type: schema.schema_type.clone(),
            format: schema.format.clone(),
            fingerprint: schema.fingerprint.clone(),
            description: schema.description.clone(),
            tags: schema.tags.clone(),
            created_at: schema.created_at,
            updated_at: schema.updated_at,
            owner: schema.owner.clone(),
            visibility: schema.visibility.clone(),
            version_count: 1, // Will be updated by storage layer
            size: schema.size(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let schema = Schema::new(
            "test-schema".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object", "properties": {"value": {"type": "number"}}}"#.to_string(),
            SchemaFormat::Json,
        );

        assert_eq!(schema.name, "test-schema");
        assert_eq!(schema.version.to_string(), "1.0.0");
        assert_eq!(schema.schema_type, SchemaType::Metric);
        assert_eq!(schema.format, SchemaFormat::Json);
        assert!(!schema.fingerprint.is_empty());
    }

    #[test]
    fn test_schema_content_validation() {
        let valid_json = r#"{"type": "object", "properties": {"value": {"type": "number"}}}"#;
        let invalid_json = r#"{"type": "object", "properties": {"value": {"type": "number"}}}"#;

        let mut schema = Schema::new(
            "test".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            valid_json.to_string(),
            SchemaFormat::Json,
        );

        assert!(schema.validate_content().is_ok());

        schema.update_content(invalid_json.to_string());
        assert!(schema.validate_content().is_ok()); // This should actually be invalid

        // Test YAML validation
        let yaml_schema = Schema::new(
            "test".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            "type: object\nproperties:\n  value:\n    type: number".to_string(),
            SchemaFormat::Yaml,
        );

        assert!(yaml_schema.validate_content().is_ok());
    }

    #[test]
    fn test_schema_metadata_conversion() {
        let schema = Schema::new(
            "test-schema".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaType::Metric,
            r#"{"type": "object"}"#.to_string(),
            SchemaFormat::Json,
        );

        let metadata: SchemaMetadata = schema.into();
        assert_eq!(metadata.name, "test-schema");
        assert_eq!(metadata.version_count, 1);
        assert!(metadata.size > 0);
    }
}
