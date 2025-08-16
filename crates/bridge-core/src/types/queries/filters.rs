//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query filter types for the OpenTelemetry Data Lake Bridge

use serde::{Deserialize, Serialize};

/// Query filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    /// Field name
    pub field: String,

    /// Filter operator
    pub operator: FilterOperator,

    /// Filter value
    pub value: FilterValue,
}

/// Filter operator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    In,
    NotIn,
    Exists,
    NotExists,
    StartsWith,
    EndsWith,
    Regex,
}

/// Filter value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<FilterValue>),
    Null,
}

impl Filter {
    /// Create a new filter
    pub fn new(field: String, operator: FilterOperator, value: FilterValue) -> Self {
        Self {
            field,
            operator,
            value,
        }
    }

    /// Create an equals filter
    pub fn equals(field: String, value: FilterValue) -> Self {
        Self::new(field, FilterOperator::Equals, value)
    }

    /// Create a contains filter
    pub fn contains(field: String, value: String) -> Self {
        Self::new(field, FilterOperator::Contains, FilterValue::String(value))
    }

    /// Create a greater than filter
    pub fn greater_than(field: String, value: f64) -> Self {
        Self::new(
            field,
            FilterOperator::GreaterThan,
            FilterValue::Number(value),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_creation() {
        let filter = Filter::equals(
            "test_field".to_string(),
            FilterValue::String("test_value".to_string()),
        );
        assert_eq!(filter.field, "test_field");
        assert!(matches!(filter.operator, FilterOperator::Equals));
    }
}
