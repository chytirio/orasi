//! Aggregation factory for creating aggregation functions

use super::trait_def::AggregationFunction;
use super::fun::{CountAggregation, SumAggregation, AverageAggregation, MinAggregation, MaxAggregation};

/// Aggregation factory for creating aggregation functions
pub struct AggregationFactory;

impl AggregationFactory {
    /// Create aggregation function by name
    pub fn create_function(name: &str) -> Option<Box<dyn AggregationFunction>> {
        match name {
            "count" => Some(Box::new(CountAggregation)),
            "sum" => Some(Box::new(SumAggregation)),
            "average" => Some(Box::new(AverageAggregation)),
            "min" => Some(Box::new(MinAggregation)),
            "max" => Some(Box::new(MaxAggregation)),
            _ => None,
        }
    }

    /// Get available function names
    pub fn get_available_functions() -> Vec<String> {
        vec![
            "count".to_string(),
            "sum".to_string(),
            "average".to_string(),
            "min".to_string(),
            "max".to_string(),
        ]
    }
}
