//! Transformation function trait definition

use async_trait::async_trait;
use bridge_core::BridgeResult;

/// Transformation function trait
#[async_trait]
pub trait TransformationFunction: Send + Sync {
    /// Get function name
    fn name(&self) -> &str;

    /// Get function type
    fn function_type(&self) -> &str;

    /// Apply transformation function
    async fn apply(&self, value: String) -> BridgeResult<String>;

    /// Get function description
    fn description(&self) -> &str;
}
