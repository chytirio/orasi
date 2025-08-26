use async_trait::async_trait;
use bridge_core::BridgeResult;

use crate::aggregations::trait_def::AggregationFunction;

/// Max aggregation function
pub struct MaxAggregation;

#[async_trait]
impl AggregationFunction for MaxAggregation {
    fn name(&self) -> &str {
        "max"
    }

    fn function_type(&self) -> &str {
        "max"
    }

    async fn apply(&self, values: Vec<f64>) -> BridgeResult<f64> {
        values.into_iter().reduce(f64::max).ok_or_else(|| {
            bridge_core::BridgeError::stream("Cannot calculate max of empty values".to_string())
        })
    }

    fn description(&self) -> &str {
        "Find the maximum value"
    }
}
