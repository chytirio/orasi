//! State configuration types

use bridge_core::BridgeResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// State configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConfig {
    /// State store type
    pub store_type: StateStoreType,

    /// State store configuration
    pub config: HashMap<String, serde_json::Value>,

    /// Enable state persistence
    pub enable_persistence: bool,

    /// State persistence path
    pub persistence_path: Option<String>,
}

impl Default for StateConfig {
    fn default() -> Self {
        Self {
            store_type: StateStoreType::InMemory,
            config: HashMap::new(),
            enable_persistence: false,
            persistence_path: None,
        }
    }
}

impl StateConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.enable_persistence && self.persistence_path.is_none() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Persistence path must be specified when persistence is enabled".to_string(),
            ));
        }

        Ok(())
    }
}

/// State store types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateStoreType {
    InMemory,
    Redis,
    Database,
    File,
    Custom(String),
}
