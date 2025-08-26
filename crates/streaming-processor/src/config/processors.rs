//! Processor configuration types

use bridge_core::BridgeResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    /// Processor type
    pub processor_type: ProcessorType,

    /// Processor name
    pub name: String,

    /// Processor version
    pub version: String,

    /// Processor-specific configuration
    pub config: HashMap<String, serde_json::Value>,

    /// Processing order
    pub order: usize,
}

impl ProcessorConfig {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.name.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Processor name cannot be empty".to_string(),
            ));
        }

        if self.version.is_empty() {
            return Err(bridge_core::error::BridgeError::configuration(
                "Processor version cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

/// Processor types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessorType {
    Filter,
    Transform,
    Aggregate,
    Enrich,
    Window,
    Custom(String),
}
