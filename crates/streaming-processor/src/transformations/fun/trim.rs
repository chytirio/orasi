//! Transformation function implementations

use async_trait::async_trait;
use bridge_core::BridgeResult;

use super::trait_def::TransformationFunction;

/// Trim transformation function
pub struct TrimTransformation;

#[async_trait]
impl TransformationFunction for TrimTransformation {
    fn name(&self) -> &str {
        "trim"
    }

    fn function_type(&self) -> &str {
        "string"
    }

    async fn apply(&self, value: String) -> BridgeResult<String> {
        Ok(value.trim().to_string())
    }

    fn description(&self) -> &str {
        "Trim whitespace from string"
    }
}
