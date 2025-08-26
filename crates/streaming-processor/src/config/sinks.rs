//! Sink configuration types

use bridge_core::BridgeResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::security::AuthConfig;
use super::connection::ConnectionConfig;

/// Sink configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkConfig {
    /// Sink type
    pub sink_type: SinkType,

    /// Sink name
    pub name: String,

    /// Sink version
    pub version: String,

    /// Sink-specific configuration
    pub config: HashMap<String, serde_json::Value>,

    /// Authentication configuration
    pub auth: Option<AuthConfig>,

    /// Connection configuration
    pub connection: ConnectionConfig,
}

impl SinkConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.name.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Sink name cannot be empty".to_string(),
            ));
        }

        if self.version.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Sink version cannot be empty".to_string(),
            ));
        }

        self.connection.validate()?;

        Ok(())
    }
}

/// Sink types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SinkType {
    Kafka,
    Http,
    File,
    Database,
    Custom(String),
}
