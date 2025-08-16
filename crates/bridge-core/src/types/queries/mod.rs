//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query structures for the OpenTelemetry Data Lake Bridge
//!
//! This module provides query-related data structures including query types,
//! filters, aggregations, and results used throughout the bridge.

pub mod queries;
pub mod results;
pub mod filters;
pub mod aggregations;
pub mod time_range;

// Re-export commonly used types
pub use queries::{MetricsQuery, TracesQuery, LogsQuery, TelemetryQuery};
pub use results::{MetricsResult, TracesResult, LogsResult, QueryStatus, QueryError};
pub use filters::{Filter, FilterOperator, FilterValue};
pub use aggregations::{Aggregation, AggregationFunction};
pub use time_range::TimeRange;
