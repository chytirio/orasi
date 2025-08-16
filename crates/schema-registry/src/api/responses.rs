//! API response structures
//!
//! This module contains all the response structures for the API endpoints.

use serde::{Deserialize, Serialize};

use crate::schema::{Schema, SchemaMetadata};

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Health status
    pub status: String,

    /// Response timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Whether registry is initialized
    pub initialized: bool,

    /// Last health check timestamp
    pub last_health_check: chrono::DateTime<chrono::Utc>,
}

/// Register schema response
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterSchemaResponse {
    /// Schema fingerprint
    pub fingerprint: String,

    /// Schema version
    pub version: String,

    /// Response message
    pub message: String,
}

/// List schemas response
#[derive(Debug, Serialize, Deserialize)]
pub struct ListSchemasResponse {
    /// List of schemas
    pub schemas: Vec<SchemaMetadata>,

    /// Total count
    pub total_count: usize,
}

/// Get schema response
#[derive(Debug, Serialize, Deserialize)]
pub struct GetSchemaResponse {
    /// Schema
    pub schema: Schema,
}

/// Delete schema response
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteSchemaResponse {
    /// Response message
    pub message: String,
}

/// Validate data response
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateDataResponse {
    /// Whether validation passed
    pub valid: bool,

    /// Validation status
    pub status: String,

    /// Number of errors
    pub error_count: usize,

    /// Number of warnings
    pub warning_count: usize,

    /// Validation errors
    pub errors: Vec<crate::validation::ValidationError>,

    /// Validation warnings
    pub warnings: Vec<crate::validation::ValidationWarning>,
}

/// Validate schema response
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateSchemaResponse {
    /// Whether validation passed
    pub valid: bool,

    /// Validation status
    pub status: String,

    /// Number of errors
    pub error_count: usize,

    /// Number of warnings
    pub warning_count: usize,

    /// Validation errors
    pub errors: Vec<crate::validation::ValidationError>,

    /// Validation warnings
    pub warnings: Vec<crate::validation::ValidationWarning>,
}

/// Evolve schema response
#[derive(Debug, Serialize, Deserialize)]
pub struct EvolveSchemaResponse {
    /// Evolved schema fingerprint
    pub fingerprint: String,

    /// Evolved schema version
    pub version: String,

    /// Response message
    pub message: String,
}

/// Metrics response
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    /// Registry metrics
    pub metrics: crate::registry::RegistryMetrics,
}

/// Stats response
#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    /// Registry statistics
    pub stats: crate::registry::RegistryStats,
}

/// Error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error type
    pub error: String,

    /// Error message
    pub message: String,

    /// HTTP status code
    pub status_code: u16,
}
