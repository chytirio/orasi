//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Delta Lake schema management
//! 
//! This module provides schema management capabilities for Delta Lake,
//! including schema evolution, validation, and mapping.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use crate::error::{DeltaLakeError, DeltaLakeResult};

/// Delta Lake schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaLakeSchema {
    /// Schema version
    pub version: u32,
    /// Schema fields
    pub fields: Vec<DeltaLakeField>,
    /// Schema metadata
    pub metadata: HashMap<String, String>,
}

/// Delta Lake field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaLakeField {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: DeltaLakeFieldType,
    /// Field nullable
    pub nullable: bool,
    /// Field metadata
    pub metadata: HashMap<String, String>,
}

/// Delta Lake field types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeltaLakeFieldType {
    /// String type
    String,
    /// Integer type
    Integer,
    /// Long type
    Long,
    /// Float type
    Float,
    /// Double type
    Double,
    /// Boolean type
    Boolean,
    /// Timestamp type
    Timestamp,
    /// Date type
    Date,
    /// Binary type
    Binary,
    /// Array type
    Array(Box<DeltaLakeFieldType>),
    /// Map type
    Map(Box<DeltaLakeFieldType>, Box<DeltaLakeFieldType>),
    /// Struct type
    Struct(Vec<DeltaLakeField>),
}

/// Delta Lake schema manager
pub struct DeltaLakeSchemaManager {
    /// Current schema
    current_schema: Option<DeltaLakeSchema>,
    /// Schema history
    schema_history: Vec<DeltaLakeSchema>,
}

impl DeltaLakeSchemaManager {
    /// Create a new schema manager
    pub fn new() -> Self {
        Self {
            current_schema: None,
            schema_history: Vec::new(),
        }
    }

    /// Get the current schema
    pub fn current_schema(&self) -> Option<&DeltaLakeSchema> {
        self.current_schema.as_ref()
    }

    /// Set the current schema
    pub fn set_schema(&mut self, schema: DeltaLakeSchema) -> DeltaLakeResult<()> {
        info!("Setting Delta Lake schema version {}", schema.version);
        
        // Validate the schema
        self.validate_schema(&schema)?;
        
        // Add to history if it's a new version
        if let Some(current) = &self.current_schema {
            if current.version != schema.version {
                self.schema_history.push(current.clone());
            }
        }
        
        self.current_schema = Some(schema);
        Ok(())
    }

    /// Validate a schema
    pub fn validate_schema(&self, schema: &DeltaLakeSchema) -> DeltaLakeResult<()> {
        debug!("Validating Delta Lake schema version {}", schema.version);
        
        // Check for duplicate field names
        let mut field_names = std::collections::HashSet::new();
        for field in &schema.fields {
            if !field_names.insert(&field.name) {
                return Err(DeltaLakeError::schema(format!("Duplicate field name: {}", field.name)));
            }
        }
        
        // Validate field types
        for field in &schema.fields {
            self.validate_field_type(&field.field_type)?;
        }
        
        info!("Schema validation completed successfully");
        Ok(())
    }

    /// Validate a field type
    fn validate_field_type(&self, field_type: &DeltaLakeFieldType) -> DeltaLakeResult<()> {
        match field_type {
            DeltaLakeFieldType::Array(element_type) => {
                self.validate_field_type(element_type)?;
            }
            DeltaLakeFieldType::Map(key_type, value_type) => {
                self.validate_field_type(key_type)?;
                self.validate_field_type(value_type)?;
            }
            DeltaLakeFieldType::Struct(fields) => {
                for field in fields {
                    self.validate_field_type(&field.field_type)?;
                }
            }
            _ => {
                // Primitive types are always valid
            }
        }
        Ok(())
    }

    /// Get schema history
    pub fn schema_history(&self) -> &[DeltaLakeSchema] {
        &self.schema_history
    }

    /// Check if schema evolution is needed
    pub fn needs_schema_evolution(&self, new_schema: &DeltaLakeSchema) -> bool {
        if let Some(current) = &self.current_schema {
            current.version != new_schema.version
        } else {
            true
        }
    }

    /// Evolve schema
    pub fn evolve_schema(&mut self, new_schema: DeltaLakeSchema) -> DeltaLakeResult<()> {
        info!("Evolving Delta Lake schema from version {} to {}", 
              self.current_schema.as_ref().map(|s| s.version).unwrap_or(0),
              new_schema.version);
        
        // Validate the new schema
        self.validate_schema(&new_schema)?;
        
        // Check for backward compatibility
        if let Some(current) = &self.current_schema {
            self.check_backward_compatibility(current, &new_schema)?;
        }
        
        // Set the new schema
        self.set_schema(new_schema)?;
        
        info!("Schema evolution completed successfully");
        Ok(())
    }

    /// Check backward compatibility
    fn check_backward_compatibility(&self, old_schema: &DeltaLakeSchema, new_schema: &DeltaLakeSchema) -> DeltaLakeResult<()> {
        debug!("Checking backward compatibility between schema versions {} and {}", 
               old_schema.version, new_schema.version);
        
        // Create a map of old fields for quick lookup
        let old_fields: HashMap<&str, &DeltaLakeField> = old_schema.fields.iter()
            .map(|f| (f.name.as_str(), f))
            .collect();
        
        // Check each new field for compatibility
        for new_field in &new_schema.fields {
            if let Some(old_field) = old_fields.get(new_field.name.as_str()) {
                // Check if the field type is compatible
                if !self.is_type_compatible(&old_field.field_type, &new_field.field_type) {
                    return Err(DeltaLakeError::schema(
                        format!("Incompatible type change for field {}: {:?} -> {:?}", 
                                new_field.name, old_field.field_type, new_field.field_type)
                    ));
                }
                
                // Check if nullable constraint is compatible
                if old_field.nullable && !new_field.nullable {
                    return Err(DeltaLakeError::schema(
                        format!("Cannot make field {} non-nullable", new_field.name)
                    ));
                }
            }
        }
        
        Ok(())
    }

    /// Check if two types are compatible
    fn is_type_compatible(&self, old_type: &DeltaLakeFieldType, new_type: &DeltaLakeFieldType) -> bool {
        match (old_type, new_type) {
            (DeltaLakeFieldType::String, DeltaLakeFieldType::String) => true,
            (DeltaLakeFieldType::Integer, DeltaLakeFieldType::Long) => true,
            (DeltaLakeFieldType::Integer, DeltaLakeFieldType::Float) => true,
            (DeltaLakeFieldType::Integer, DeltaLakeFieldType::Double) => true,
            (DeltaLakeFieldType::Long, DeltaLakeFieldType::Double) => true,
            (DeltaLakeFieldType::Float, DeltaLakeFieldType::Double) => true,
            (DeltaLakeFieldType::Array(old_element), DeltaLakeFieldType::Array(new_element)) => {
                self.is_type_compatible(old_element, new_element)
            }
            (DeltaLakeFieldType::Map(old_key, old_value), DeltaLakeFieldType::Map(new_key, new_value)) => {
                self.is_type_compatible(old_key, new_key) && self.is_type_compatible(old_value, new_value)
            }
            (DeltaLakeFieldType::Struct(old_fields), DeltaLakeFieldType::Struct(new_fields)) => {
                // For structs, we need to check field-by-field compatibility
                if old_fields.len() != new_fields.len() {
                    return false;
                }
                
                for (old_field, new_field) in old_fields.iter().zip(new_fields.iter()) {
                    if old_field.name != new_field.name || 
                       !self.is_type_compatible(&old_field.field_type, &new_field.field_type) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }
}

impl Default for DeltaLakeSchemaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_manager_creation() {
        let manager = DeltaLakeSchemaManager::new();
        assert!(manager.current_schema().is_none());
        assert!(manager.schema_history().is_empty());
    }

    #[test]
    fn test_schema_validation() {
        let mut manager = DeltaLakeSchemaManager::new();
        
        let schema = DeltaLakeSchema {
            version: 1,
            fields: vec![
                DeltaLakeField {
                    name: "id".to_string(),
                    field_type: DeltaLakeFieldType::Long,
                    nullable: false,
                    metadata: HashMap::new(),
                },
                DeltaLakeField {
                    name: "name".to_string(),
                    field_type: DeltaLakeFieldType::String,
                    nullable: true,
                    metadata: HashMap::new(),
                },
            ],
            metadata: HashMap::new(),
        };
        
        assert!(manager.validate_schema(&schema).is_ok());
    }

    #[test]
    fn test_duplicate_field_validation() {
        let manager = DeltaLakeSchemaManager::new();
        
        let schema = DeltaLakeSchema {
            version: 1,
            fields: vec![
                DeltaLakeField {
                    name: "id".to_string(),
                    field_type: DeltaLakeFieldType::Long,
                    nullable: false,
                    metadata: HashMap::new(),
                },
                DeltaLakeField {
                    name: "id".to_string(), // Duplicate name
                    field_type: DeltaLakeFieldType::String,
                    nullable: true,
                    metadata: HashMap::new(),
                },
            ],
            metadata: HashMap::new(),
        };
        
        assert!(manager.validate_schema(&schema).is_err());
    }
}
