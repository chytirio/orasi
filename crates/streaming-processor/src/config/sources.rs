//! Source configuration types

use bridge_core::BridgeResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::security::AuthConfig;
use super::connection::ConnectionConfig;

/// Source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Source type
    pub source_type: SourceType,

    /// Source name
    pub name: String,

    /// Source version
    pub version: String,

    /// Source-specific configuration
    pub config: HashMap<String, serde_json::Value>,

    /// Authentication configuration
    pub auth: Option<AuthConfig>,

    /// Connection configuration
    pub connection: ConnectionConfig,
}

impl SourceConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.name.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Source name cannot be empty".to_string(),
            ));
        }

        if self.version.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Source version cannot be empty".to_string(),
            ));
        }

        self.connection.validate()?;

        Ok(())
    }
}

/// Source types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    Kafka,
    Http,
    File,
    WebSocket,
    Custom(String),
}
