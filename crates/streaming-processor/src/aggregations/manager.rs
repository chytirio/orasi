//! Aggregation manager for managing aggregation functions

use bridge_core::BridgeResult;
use std::collections::HashMap;

use super::trait_def::AggregationFunction;
use super::fun::{CountAggregation, SumAggregation, AverageAggregation, MinAggregation, MaxAggregation};

/// Aggregation manager for managing aggregation functions
pub struct AggregationManager {
    functions: HashMap<String, Box<dyn AggregationFunction>>,
}

impl AggregationManager {
    /// Create new aggregation manager
    pub fn new() -> Self {
        let mut manager = Self {
            functions: HashMap::new(),
        };

        // Register default aggregation functions
        manager.register_function(Box::new(CountAggregation));
        manager.register_function(Box::new(SumAggregation));
        manager.register_function(Box::new(AverageAggregation));
        manager.register_function(Box::new(MinAggregation));
        manager.register_function(Box::new(MaxAggregation));

        manager
    }

    /// Register aggregation function
    pub fn register_function(&mut self, function: Box<dyn AggregationFunction>) {
        self.functions.insert(function.name().to_string(), function);
    }

    /// Get aggregation function
    pub fn get_function(&self, name: &str) -> Option<&dyn AggregationFunction> {
        self.functions.get(name).map(|f| f.as_ref())
    }

    /// Get all function names
    pub fn get_function_names(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }

    /// Apply aggregation function
    pub async fn apply_function(&self, name: &str, values: Vec<f64>) -> BridgeResult<f64> {
        if let Some(function) = self.get_function(name) {
            function.apply(values).await
        } else {
            Err(bridge_core::BridgeError::configuration(format!(
                "Unknown aggregation function: {}",
                name
            )))
        }
    }
}
