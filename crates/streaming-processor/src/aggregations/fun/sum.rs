use async_trait::async_trait;
use bridge_core::BridgeResult;

use crate::aggregations::trait_def::AggregationFunction;

/// Sum aggregation function
pub struct SumAggregation;

#[async_trait]
impl AggregationFunction for SumAggregation {
    fn name(&self) -> &str {
        "sum"
    }

    fn function_type(&self) -> &str {
        "sum"
    }

    async fn apply(&self, values: Vec<f64>) -> BridgeResult<f64> {
        Ok(values.iter().sum())
    }

    fn description(&self) -> &str {
        "Sum all values"
    }
}
