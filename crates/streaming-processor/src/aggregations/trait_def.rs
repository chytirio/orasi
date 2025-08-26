//! Aggregation function trait definition

use async_trait::async_trait;
use bridge_core::BridgeResult;

/// Aggregation function trait
#[async_trait]
pub trait AggregationFunction: Send + Sync {
    /// Get function name
    fn name(&self) -> &str;

    /// Get function type
    fn function_type(&self) -> &str;

    /// Apply aggregation function
    async fn apply(&self, values: Vec<f64>) -> BridgeResult<f64>;

    /// Get function description
    fn description(&self) -> &str;
}
