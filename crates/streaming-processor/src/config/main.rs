//! Main streaming processor configuration

use bridge_core::BridgeResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::sources::SourceConfig;
use super::processors::ProcessorConfig;
use super::sinks::SinkConfig;
use super::processing::ProcessingConfig;
use super::state::StateConfig;
use super::metrics::MetricsConfig;
use super::security::SecurityConfig;

/// Streaming processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingProcessorConfig {
    /// Processor name
    pub name: String,

    /// Processor version
    pub version: String,

    /// Sources configuration
    pub sources: HashMap<String, SourceConfig>,

    /// Processors configuration
    pub processors: Vec<ProcessorConfig>,

    /// Sinks configuration
    pub sinks: HashMap<String, SinkConfig>,

    /// General processing configuration
    pub processing: ProcessingConfig,

    /// State management configuration
    pub state: StateConfig,

    /// Metrics configuration
    pub metrics: MetricsConfig,

    /// Security configuration
    pub security: SecurityConfig,
}

impl Default for StreamingProcessorConfig {
    fn default() -> Self {
        Self {
            name: "streaming-processor".to_string(),
            version: "1.0.0".to_string(),
            sources: HashMap::new(),
            processors: Vec::new(),
            sinks: HashMap::new(),
            processing: ProcessingConfig::default(),
            state: StateConfig::default(),
            metrics: MetricsConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl StreamingProcessorConfig {
    /// Validate the configuration
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

        // Validate sources
        for (name, source_config) in &self.sources {
            source_config.validate().map_err(|e| {
                bridge_core::error::BridgeError::configuration(format!(
                    "Source '{}' configuration error: {}",
                    name, e
                ))
            })?;
        }

        // Validate processors
        for processor_config in &self.processors {
            processor_config.validate().map_err(|e| {
                bridge_core::error::BridgeError::configuration(format!(
                    "Processor configuration error: {}",
                    e
                ))
            })?;
        }

        // Validate sinks
        for (name, sink_config) in &self.sinks {
            sink_config.validate().map_err(|e| {
                bridge_core::error::BridgeError::configuration(format!(
                    "Sink '{}' configuration error: {}",
                    name, e
                ))
            })?;
        }

        // Validate processing configuration
        self.processing.validate()?;

        // Validate state configuration
        self.state.validate()?;

        // Validate metrics configuration
        self.metrics.validate()?;

        // Validate security configuration
        self.security.validate()?;

        Ok(())
    }
}
