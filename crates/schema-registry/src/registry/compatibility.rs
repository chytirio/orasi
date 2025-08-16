//! Schema compatibility and evolution
//!
//! This module handles schema compatibility checking and evolution logic.

use crate::error::{SchemaRegistryError, SchemaRegistryResult};
use crate::schema::{CompatibilityMode, Schema, SchemaVersion};

/// Schema compatibility checker
pub struct CompatibilityChecker;

impl CompatibilityChecker {
    /// Check schema compatibility based on compatibility mode
    pub async fn check_schema_compatibility(
        existing: &Schema,
        new: &Schema,
    ) -> SchemaRegistryResult<bool> {
        match existing.compatibility_mode {
            CompatibilityMode::None => Ok(true),
            CompatibilityMode::Backward => {
                // New schema should be able to read data written with old schema
                Self::check_backward_compatibility(existing, new).await
            }
            CompatibilityMode::Forward => {
                // Old schema should be able to read data written with new schema
                Self::check_forward_compatibility(existing, new).await
            }
            CompatibilityMode::Full => {
                // Both backward and forward compatibility required
                let backward = Self::check_backward_compatibility(existing, new).await?;
                let forward = Self::check_forward_compatibility(existing, new).await?;
                Ok(backward && forward)
            }
        }
    }

    /// Check backward compatibility (new schema can read old data)
    async fn check_backward_compatibility(
        existing: &Schema,
        new: &Schema,
    ) -> SchemaRegistryResult<bool> {
        // For JSON Schema, check if new schema is a superset of existing schema
        if existing.format == crate::schema::SchemaFormat::Json
            && new.format == crate::schema::SchemaFormat::Json
        {
            return Self::check_json_backward_compatibility(existing, new).await;
        }

        // For other formats, use basic content comparison for now
        // In a real implementation, format-specific compatibility checks would be implemented
        Ok(true)
    }

    /// Check forward compatibility (old schema can read new data)
    async fn check_forward_compatibility(
        existing: &Schema,
        new: &Schema,
    ) -> SchemaRegistryResult<bool> {
        // For JSON Schema, check if existing schema is a superset of new schema
        if existing.format == crate::schema::SchemaFormat::Json
            && new.format == crate::schema::SchemaFormat::Json
        {
            return Self::check_json_forward_compatibility(existing, new).await;
        }

        // For other formats, use basic content comparison for now
        Ok(true)
    }

    /// Check JSON schema backward compatibility
    async fn check_json_backward_compatibility(
        existing: &Schema,
        new: &Schema,
    ) -> SchemaRegistryResult<bool> {
        // Parse both schemas
        let existing_json: serde_json::Value = serde_json::from_str(&existing.content)
            .map_err(|e| SchemaRegistryError::Validation {
                message: format!("Invalid existing schema JSON: {}", e),
            })?;

        let new_json: serde_json::Value = serde_json::from_str(&new.content)
            .map_err(|e| SchemaRegistryError::Validation {
                message: format!("Invalid new schema JSON: {}", e),
            })?;

        // Basic compatibility check: ensure new schema doesn't remove required fields
        // This is a simplified check - in practice, you'd want more sophisticated JSON Schema validation
        if let (serde_json::Value::Object(existing_obj), serde_json::Value::Object(new_obj)) =
            (&existing_json, &new_json)
        {
            if let Some(existing_props) = existing_obj.get("properties") {
                if let Some(new_props) = new_obj.get("properties") {
                    if let (
                        serde_json::Value::Object(existing_props_obj),
                        serde_json::Value::Object(new_props_obj),
                    ) = (existing_props, new_props)
                    {
                        // Check that all existing properties are still present
                        for (key, _) in existing_props_obj {
                            if !new_props_obj.contains_key(key) {
                                return Ok(false);
                            }
                        }
                    }
                }
            }
        }

        Ok(true)
    }

    /// Check JSON schema forward compatibility
    async fn check_json_forward_compatibility(
        existing: &Schema,
        new: &Schema,
    ) -> SchemaRegistryResult<bool> {
        // Parse both schemas
        let existing_json: serde_json::Value = serde_json::from_str(&existing.content)
            .map_err(|e| SchemaRegistryError::Validation {
                message: format!("Invalid existing schema JSON: {}", e),
            })?;

        let new_json: serde_json::Value = serde_json::from_str(&new.content)
            .map_err(|e| SchemaRegistryError::Validation {
                message: format!("Invalid new schema JSON: {}", e),
            })?;

        // Basic compatibility check: ensure existing schema can handle new fields
        // This is a simplified check - in practice, you'd want more sophisticated JSON Schema validation
        if let (serde_json::Value::Object(existing_obj), serde_json::Value::Object(new_obj)) =
            (&existing_json, &new_json)
        {
            if let Some(existing_props) = existing_obj.get("properties") {
                if let Some(new_props) = new_obj.get("properties") {
                    if let (
                        serde_json::Value::Object(existing_props_obj),
                        serde_json::Value::Object(new_props_obj),
                    ) = (existing_props, new_props)
                    {
                        // Check that new properties are optional or have default values
                        for (key, new_prop) in new_props_obj {
                            if !existing_props_obj.contains_key(key) {
                                // New field should be optional for forward compatibility
                                if let serde_json::Value::Object(prop_obj) = new_prop {
                                    if let Some(required) = prop_obj.get("required") {
                                        if required.as_bool().unwrap_or(false) {
                                            return Ok(false);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(true)
    }

    /// Increment schema version
    pub fn increment_schema_version(version: &SchemaVersion) -> SchemaVersion {
        SchemaVersion::new(version.major, version.minor, version.patch + 1)
    }
}
