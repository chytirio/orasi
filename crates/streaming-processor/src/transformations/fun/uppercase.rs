//! Transformation function implementations

use async_trait::async_trait;
use bridge_core::BridgeResult;

use super::trait_def::TransformationFunction;

/// Uppercase transformation function
pub struct UppercaseTransformation;

#[async_trait]
impl TransformationFunction for UppercaseTransformation {
    fn name(&self) -> &str {
        "uppercase"
    }

    fn function_type(&self) -> &str {
        "string"
    }

    async fn apply(&self, value: String) -> BridgeResult<String> {
        Ok(value.to_uppercase())
    }

    fn description(&self) -> &str {
        "Convert string to uppercase"
    }
}
