use async_trait::async_trait;
use bridge_core::BridgeResult;

use crate::aggregations::trait_def::AggregationFunction;

/// Min aggregation function
pub struct MinAggregation;

#[async_trait]
impl AggregationFunction for MinAggregation {
    fn name(&self) -> &str {
        "min"
    }

    fn function_type(&self) -> &str {
        "min"
    }

    async fn apply(&self, values: Vec<f64>) -> BridgeResult<f64> {
        values.into_iter().reduce(f64::min).ok_or_else(|| {
            bridge_core::BridgeError::stream("Cannot calculate min of empty values".to_string())
        })
    }

    fn description(&self) -> &str {
        "Find the minimum value"
    }
}
