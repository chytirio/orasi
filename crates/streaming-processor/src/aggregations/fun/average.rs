use async_trait::async_trait;
use bridge_core::BridgeResult;

use crate::aggregations::trait_def::AggregationFunction;

/// Average aggregation function
pub struct AverageAggregation;

#[async_trait]
impl AggregationFunction for AverageAggregation {
    fn name(&self) -> &str {
        "average"
    }

    fn function_type(&self) -> &str {
        "average"
    }

    async fn apply(&self, values: Vec<f64>) -> BridgeResult<f64> {
        if values.is_empty() {
            return Err(bridge_core::BridgeError::stream(
                "Cannot calculate average of empty values".to_string(),
            ));
        }

        let sum: f64 = values.iter().sum();
        Ok(sum / values.len() as f64)
    }

    fn description(&self) -> &str {
        "Calculate the average of all values"
    }
}
