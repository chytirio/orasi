//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Main bridge configuration for the OpenTelemetry Data Lake Bridge
//!
//! This module provides the main configuration structure and utilities.

use crate::error::{BridgeError, BridgeResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use validator::Validate;

use super::{
    AdvancedConfig, IngestionConfig, LakehouseConfig, MonitoringConfig, ProcessingConfig,
    SecurityConfig,
};

/// Default configuration file path
pub const DEFAULT_CONFIG_PATH: &str = "config/bridge.toml";

/// Main bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BridgeConfig {
    /// Bridge instance name
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    /// Bridge version
    pub version: Option<String>,

    /// Environment (development, staging, production)
    #[validate(length(min = 1, max = 20))]
    pub environment: String,

    /// Ingestion configuration
    pub ingestion: IngestionConfig,

    /// Lakehouse configurations
    // #[validate(length(min = 1))]  // Commented out for Phase 1 - allow empty lakehouses
    pub lakehouses: HashMap<String, LakehouseConfig>,

    /// Processing configuration
    pub processing: ProcessingConfig,

    /// Security configuration
    pub security: SecurityConfig,

    /// Monitoring configuration
    pub monitoring: MonitoringConfig,

    /// Advanced configuration options
    pub advanced: AdvancedConfig,
}

impl BridgeConfig {
    /// Load configuration from file
    pub fn from_file(path: &PathBuf) -> BridgeResult<Self> {
        let config = config::Config::builder()
            .add_source(config::File::from(path.as_path()))
            .add_source(config::Environment::with_prefix("BRIDGE"))
            .build()
            .map_err(|e| {
                BridgeError::configuration_with_source("Failed to load configuration", e)
            })?;

        let bridge_config: BridgeConfig = config.try_deserialize().map_err(|e| {
            BridgeError::configuration_with_source("Failed to deserialize configuration", e)
        })?;

        bridge_config.validate().map_err(|e| {
            BridgeError::validation_with_source("Configuration validation failed", e)
        })?;

        Ok(bridge_config)
    }

    /// Load configuration from string
    pub fn from_str(content: &str) -> BridgeResult<Self> {
        let config: BridgeConfig = serde_json::from_str(content).map_err(|e| {
            BridgeError::serialization_with_source("Failed to serialize configuration", e)
        })?;

        config.validate().map_err(|e| {
            BridgeError::validation_with_source("Configuration validation failed", e)
        })?;

        Ok(config)
    }

    /// Get default configuration
    pub fn default() -> Self {
        Self {
            name: "orasi".to_string(),
            version: Some("0.1.0".to_string()),
            environment: "development".to_string(),
            ingestion: IngestionConfig::default(),
            lakehouses: HashMap::new(),
            processing: ProcessingConfig::default(),
            security: SecurityConfig::default(),
            monitoring: MonitoringConfig::default(),
            advanced: AdvancedConfig::default(),
        }
    }

    /// Get configuration for a specific lakehouse
    pub fn get_lakehouse_config(&self, name: &str) -> Option<&LakehouseConfig> {
        self.lakehouses.get(name)
    }

    /// Validate the configuration
    pub fn validate_config(&self) -> BridgeResult<()> {
        self.validate().map_err(|e| {
            BridgeError::validation_with_source("Configuration validation failed", e)
        })?;

        // Additional custom validation logic can be added here
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BridgeConfig::default();
        assert_eq!(config.name, "orasi");
        assert_eq!(config.environment, "development");
    }

    #[test]
    fn test_config_validation() {
        let config = BridgeConfig::default();
        let result = config.validate_config();
        assert!(result.is_ok());
    }
}
