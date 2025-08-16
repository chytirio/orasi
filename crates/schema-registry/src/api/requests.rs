//! API request structures
//!
//! This module contains all the request structures for the API endpoints.

use serde::{Deserialize, Serialize};

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

/// Validate data request
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateDataRequest {
    /// Telemetry data to validate
    pub data: bridge_core::types::TelemetryBatch,
}

/// Validate schema request
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateSchemaRequest {
    /// Schema to validate
    pub schema: Schema,
}

/// Evolve schema request
#[derive(Debug, Serialize, Deserialize)]
pub struct EvolveSchemaRequest {
    /// Schema to evolve
    pub schema: Schema,
}
