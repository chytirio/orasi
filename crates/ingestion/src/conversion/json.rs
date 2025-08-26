//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! JSON converter implementation

use bridge_core::{BridgeResult, TelemetryBatch};

use super::converter::TelemetryConverter;

/// JSON converter implementation
pub struct JsonConverter;

#[async_trait::async_trait]
impl TelemetryConverter for JsonConverter {
    async fn convert(
        &self,
        data: &[u8],
        source_format: &str,
        target_format: &str,
    ) -> BridgeResult<Vec<u8>> {
        match (source_format, target_format) {
            ("json", "internal") => {
                let batch = self.convert_from_json(data).await?;
                // Serialize to internal format using serde_json
                let serialized = serde_json::to_vec(&batch).map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to serialize TelemetryBatch to JSON: {}",
                        e
                    ))
                })?;
                Ok(serialized)
            }
            ("internal", "json") => {
                // Deserialize from internal format using serde_json
                let batch: TelemetryBatch = serde_json::from_slice(data).map_err(|e| {
                    bridge_core::BridgeError::serialization(format!(
                        "Failed to deserialize TelemetryBatch from JSON: {}",
                        e
                    ))
                })?;
                self.convert_to_json(&batch).await
            }
            _ => Err(bridge_core::BridgeError::configuration(format!(
                "Unsupported conversion: {} -> {}",
                source_format, target_format
            ))),
        }
    }

    fn supported_source_formats(&self) -> Vec<String> {
        vec!["json".to_string(), "internal".to_string()]
    }

    fn supported_target_formats(&self) -> Vec<String> {
        vec!["json".to_string(), "internal".to_string()]
    }
}

impl JsonConverter {
    /// Create new JSON converter
    pub fn new() -> Self {
        Self
    }

    /// Convert JSON data to internal format
    pub async fn convert_from_json(&self, data: &[u8]) -> BridgeResult<TelemetryBatch> {
        // Parse JSON data and convert to TelemetryBatch
        let batch: TelemetryBatch = serde_json::from_slice(data).map_err(|e| {
            bridge_core::BridgeError::serialization(format!("Failed to parse JSON data: {}", e))
        })?;

        Ok(batch)
    }

    /// Convert internal format to JSON
    pub async fn convert_to_json(&self, batch: &TelemetryBatch) -> BridgeResult<Vec<u8>> {
        // Convert TelemetryBatch to JSON format
        let json_data = serde_json::to_vec_pretty(batch).map_err(|e| {
            bridge_core::BridgeError::serialization(format!(
                "Failed to serialize TelemetryBatch to JSON: {}",
                e
            ))
        })?;

        Ok(json_data)
    }
}
