//! Schema type definitions
//!
//! This module contains the enum definitions for schema types, formats,
//! visibility, and compatibility modes.

use serde::{Deserialize, Serialize};

/// Schema type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SchemaType {
    /// Metric schema
    Metric,

    /// Trace schema
    Trace,

    /// Log schema
    Log,

    /// Event schema
    Event,
}

impl std::fmt::Display for SchemaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaType::Metric => write!(f, "metric"),
            SchemaType::Trace => write!(f, "trace"),
            SchemaType::Log => write!(f, "log"),
            SchemaType::Event => write!(f, "event"),
        }
    }
}

impl std::str::FromStr for SchemaType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "metric" => Ok(SchemaType::Metric),
            "trace" => Ok(SchemaType::Trace),
            "log" => Ok(SchemaType::Log),
            "event" => Ok(SchemaType::Event),
            _ => Err(format!("Unknown schema type: {}", s)),
        }
    }
}

/// Schema format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SchemaFormat {
    /// JSON Schema
    Json,

    /// YAML Schema
    Yaml,

    /// Avro Schema
    Avro,

    /// Protocol Buffers
    Protobuf,

    /// OpenAPI/Swagger
    OpenApi,
}

impl std::fmt::Display for SchemaFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaFormat::Json => write!(f, "json"),
            SchemaFormat::Yaml => write!(f, "yaml"),
            SchemaFormat::Avro => write!(f, "avro"),
            SchemaFormat::Protobuf => write!(f, "protobuf"),
            SchemaFormat::OpenApi => write!(f, "openapi"),
        }
    }
}

impl std::str::FromStr for SchemaFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(SchemaFormat::Json),
            "yaml" => Ok(SchemaFormat::Yaml),
            "avro" => Ok(SchemaFormat::Avro),
            "protobuf" => Ok(SchemaFormat::Protobuf),
            "openapi" => Ok(SchemaFormat::OpenApi),
            _ => Err(format!("Unknown schema format: {}", s)),
        }
    }
}

/// Schema visibility
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SchemaVisibility {
    /// Public schema (visible to all)
    Public,

    /// Private schema (visible only to owner)
    Private,

    /// Organization schema (visible to organization members)
    Organization,
}

impl std::fmt::Display for SchemaVisibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaVisibility::Public => write!(f, "public"),
            SchemaVisibility::Private => write!(f, "private"),
            SchemaVisibility::Organization => write!(f, "organization"),
        }
    }
}

impl std::str::FromStr for SchemaVisibility {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "public" => Ok(SchemaVisibility::Public),
            "private" => Ok(SchemaVisibility::Private),
            "organization" => Ok(SchemaVisibility::Organization),
            _ => Err(format!("Unknown visibility: {}", s)),
        }
    }
}

/// Schema compatibility mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompatibilityMode {
    /// Backward compatibility (new schema can read old data)
    Backward,

    /// Forward compatibility (old schema can read new data)
    Forward,

    /// Full compatibility (both backward and forward)
    Full,

    /// No compatibility requirements
    None,
}

impl std::fmt::Display for CompatibilityMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompatibilityMode::Backward => write!(f, "backward"),
            CompatibilityMode::Forward => write!(f, "forward"),
            CompatibilityMode::Full => write!(f, "full"),
            CompatibilityMode::None => write!(f, "none"),
        }
    }
}

impl std::str::FromStr for CompatibilityMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "backward" => Ok(CompatibilityMode::Backward),
            "forward" => Ok(CompatibilityMode::Forward),
            "full" => Ok(CompatibilityMode::Full),
            "none" => Ok(CompatibilityMode::None),
            _ => Err(format!("Unknown compatibility mode: {}", s)),
        }
    }
}
