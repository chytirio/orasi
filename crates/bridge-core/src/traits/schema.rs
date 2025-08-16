//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Schema registry traits for the OpenTelemetry Data Lake Bridge
//!
//! This module provides traits and types for schema registry functionality.

use crate::error::BridgeResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema registry trait for managing data schemas
#[async_trait]
pub trait SchemaRegistry: Send + Sync {
    /// Register a schema
    async fn register_schema(&self, schema: Schema) -> BridgeResult<SchemaId>;

    /// Get a schema by ID
    async fn get_schema(&self, schema_id: SchemaId) -> BridgeResult<Option<Schema>>;

    /// Get schema version
    async fn get_schema_version(
        &self,
        schema_id: SchemaId,
        version: u32,
    ) -> BridgeResult<Option<Schema>>;

    /// List schemas
    async fn list_schemas(&self) -> BridgeResult<Vec<SchemaInfo>>;

    /// Delete a schema
    async fn delete_schema(&self, schema_id: SchemaId) -> BridgeResult<bool>;

    /// Validate data against schema
    async fn validate_data(
        &self,
        schema_id: SchemaId,
        data: &[u8],
    ) -> BridgeResult<ValidationResult>;

    /// Get registry statistics
    async fn get_stats(&self) -> BridgeResult<SchemaRegistryStats>;

    /// Shutdown the registry
    async fn shutdown(&self) -> BridgeResult<()>;
}

/// Schema identifier
pub type SchemaId = String;

/// Schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Schema ID
    pub id: SchemaId,

    /// Schema version
    pub version: u32,

    /// Schema name
    pub name: String,

    /// Schema description
    pub description: Option<String>,

    /// Schema format
    pub format: SchemaFormat,

    /// Schema definition
    pub definition: serde_json::Value,

    /// Schema metadata
    pub metadata: HashMap<String, String>,

    /// Schema creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Schema update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Schema format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchemaFormat {
    JsonSchema,
    Avro,
    Protobuf,
    Custom(String),
}

/// Schema information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    /// Schema ID
    pub id: SchemaId,

    /// Schema name
    pub name: String,

    /// Schema version
    pub version: u32,

    /// Schema format
    pub format: SchemaFormat,

    /// Schema creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Validation success
    pub valid: bool,

    /// Validation errors
    pub errors: Vec<ValidationError>,

    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error path
    pub path: String,

    /// Error message
    pub message: String,

    /// Error code
    pub code: String,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning path
    pub path: String,

    /// Warning message
    pub message: String,

    /// Warning code
    pub code: String,
}

/// Schema registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRegistryStats {
    /// Total schemas
    pub total_schemas: u64,

    /// Total schema versions
    pub total_versions: u64,

    /// Total validations
    pub total_validations: u64,

    /// Validations in last minute
    pub validations_per_minute: u64,

    /// Average validation time in milliseconds
    pub avg_validation_time_ms: f64,

    /// Error count
    pub error_count: u64,

    /// Last operation timestamp
    pub last_operation_time: Option<chrono::DateTime<chrono::Utc>>,
}
