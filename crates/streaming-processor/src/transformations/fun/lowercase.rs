//! Transformation function implementations

use async_trait::async_trait;
use bridge_core::BridgeResult;

use super::trait_def::TransformationFunction;

/// Lowercase transformation function
pub struct LowercaseTransformation;

#[async_trait]
impl TransformationFunction for LowercaseTransformation {
    fn name(&self) -> &str {
        "lowercase"
    }

    fn function_type(&self) -> &str {
        "string"
    }

    async fn apply(&self, value: String) -> BridgeResult<String> {
        Ok(value.to_lowercase())
    }

    fn description(&self) -> &str {
        "Convert string to lowercase"
    }
}
