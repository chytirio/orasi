//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query aggregation types for the OpenTelemetry Data Lake Bridge

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Query aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aggregation {
    /// Field name
    pub field: String,

    /// Aggregation function
    pub function: AggregationFunction,

    /// Aggregation alias
    pub alias: Option<String>,

    /// Aggregation parameters
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

/// Aggregation function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationFunction {
    Count,
    Sum,
    Average,
    Min,
    Max,
    Median,
    Percentile,
    Histogram,
    Cardinality,
    Custom(String),
}

impl Aggregation {
    /// Create a new aggregation
    pub fn new(field: String, function: AggregationFunction) -> Self {
        Self {
            field,
            function,
            alias: None,
            parameters: None,
        }
    }

    /// Create a count aggregation
    pub fn count(field: String) -> Self {
        Self::new(field, AggregationFunction::Count)
    }

    /// Create a sum aggregation
    pub fn sum(field: String) -> Self {
        Self::new(field, AggregationFunction::Sum)
    }

    /// Create an average aggregation
    pub fn average(field: String) -> Self {
        Self::new(field, AggregationFunction::Average)
    }

    /// Set aggregation alias
    pub fn with_alias(mut self, alias: String) -> Self {
        self.alias = Some(alias);
        self
    }

    /// Set aggregation parameters
    pub fn with_parameters(mut self, parameters: HashMap<String, serde_json::Value>) -> Self {
        self.parameters = Some(parameters);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregation_creation() {
        let aggregation = Aggregation::count("test_field".to_string());
        assert_eq!(aggregation.field, "test_field");
        assert!(matches!(aggregation.function, AggregationFunction::Count));
    }
}
