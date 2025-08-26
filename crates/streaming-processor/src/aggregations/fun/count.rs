use async_trait::async_trait;
use bridge_core::BridgeResult;

use crate::aggregations::trait_def::AggregationFunction;

/// Count aggregation function
pub struct CountAggregation;

#[async_trait]
impl AggregationFunction for CountAggregation {
    fn name(&self) -> &str {
        "count"
    }

    fn function_type(&self) -> &str {
        "count"
    }

    async fn apply(&self, values: Vec<f64>) -> BridgeResult<f64> {
        Ok(values.len() as f64)
    }

    fn description(&self) -> &str {
        "Count the number of values"
    }
}
