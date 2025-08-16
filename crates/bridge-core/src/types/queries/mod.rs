//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query structures for the OpenTelemetry Data Lake Bridge
//!
//! This module provides query-related data structures including query types,
//! filters, aggregations, and results used throughout the bridge.

pub mod aggregations;
pub mod filters;
pub mod queries;
pub mod results;
pub mod time_range;

// Re-export commonly used types
pub use aggregations::{Aggregation, AggregationFunction};
pub use filters::{Filter, FilterOperator, FilterValue};
pub use queries::{LogsQuery, MetricsQuery, TelemetryQuery, TracesQuery};
pub use results::{LogsResult, MetricsResult, QueryError, QueryStatus, TracesResult};
pub use time_range::TimeRange;
