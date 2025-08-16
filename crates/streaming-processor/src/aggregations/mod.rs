//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Streaming data aggregations for the bridge
//!
//! This module provides aggregation functionality for streaming data
//! including various aggregation functions and managers.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use std::collections::HashMap;

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
