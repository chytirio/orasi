//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Core type definitions for the OpenTelemetry Data Lake Bridge
//!
//! This module provides the fundamental data structures for telemetry data,
//! queries, analytics, and processing results used throughout the bridge.

pub mod analytics;
pub mod logs;
pub mod metrics;
pub mod processing;
pub mod queries;
pub mod telemetry;
pub mod traces;

// Re-export commonly used types
pub use analytics::{AnalyticsRequest, AnalyticsResponse, AnalyticsType, AnalyticsQuery, OutputFormat};
pub use logs::*;
pub use metrics::*;
pub use processing::*;
pub use queries::{MetricsQuery, TracesQuery, LogsQuery, TelemetryQuery, queries::TelemetryQueryType, MetricsResult, TracesResult, LogsResult, QueryStatus, QueryError, Filter, FilterOperator, FilterValue, Aggregation, AggregationFunction, TimeRange};
pub use telemetry::*;
pub use traces::*;
