//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query types for the OpenTelemetry Data Lake Bridge

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::{aggregations::Aggregation, filters::Filter, time_range::TimeRange};

/// Query structures for lakehouse operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsQuery {
    /// Query ID
    pub id: Uuid,

    /// Query timestamp
    pub timestamp: DateTime<Utc>,

    /// Time range
    pub time_range: TimeRange,

    /// Query filters
    pub filters: Vec<Filter>,

    /// Query aggregations
    pub aggregations: Vec<Aggregation>,

    /// Query limit
    pub limit: Option<usize>,

    /// Query offset
    pub offset: Option<usize>,

    /// Query metadata
    pub metadata: HashMap<String, String>,
}

/// Traces query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracesQuery {
    /// Query ID
    pub id: Uuid,

    /// Query timestamp
    pub timestamp: DateTime<Utc>,

    /// Time range
    pub time_range: TimeRange,

    /// Query filters
    pub filters: Vec<Filter>,

    /// Trace ID filter
    pub trace_id: Option<String>,

    /// Span ID filter
    pub span_id: Option<String>,

    /// Service name filter
    pub service_name: Option<String>,

    /// Operation name filter
    pub operation_name: Option<String>,

    /// Query limit
    pub limit: Option<usize>,

    /// Query offset
    pub offset: Option<usize>,

    /// Query metadata
    pub metadata: HashMap<String, String>,
}

/// Logs query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsQuery {
    /// Query ID
    pub id: Uuid,

    /// Query timestamp
    pub timestamp: DateTime<Utc>,

    /// Time range
    pub time_range: TimeRange,

    /// Query filters
    pub filters: Vec<Filter>,

    /// Log level filter
    pub log_level: Option<String>,

    /// Service name filter
    pub service_name: Option<String>,

    /// Query limit
    pub limit: Option<usize>,

    /// Query offset
    pub offset: Option<usize>,

    /// Query metadata
    pub metadata: HashMap<String, String>,
}

/// Generic telemetry query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryQuery {
    /// Query ID
    pub id: Uuid,

    /// Query timestamp
    pub timestamp: DateTime<Utc>,

    /// Query type
    pub query_type: TelemetryQueryType,

    /// Time range
    pub time_range: TimeRange,

    /// Query filters
    pub filters: Vec<Filter>,

    /// Query aggregations
    pub aggregations: Vec<Aggregation>,

    /// Query limit
    pub limit: Option<usize>,

    /// Query offset
    pub offset: Option<usize>,

    /// Query metadata
    pub metadata: HashMap<String, String>,
}

/// Telemetry query type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TelemetryQueryType {
    Metrics,
    Traces,
    Logs,
    All,
}

impl MetricsQuery {
    /// Create a new metrics query
    pub fn new(time_range: TimeRange) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            time_range,
            filters: Vec::new(),
            aggregations: Vec::new(),
            limit: None,
            offset: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a filter to the query
    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Add an aggregation to the query
    pub fn with_aggregation(mut self, aggregation: Aggregation) -> Self {
        self.aggregations.push(aggregation);
        self
    }

    /// Set the query limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the query offset
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Add metadata to the query
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

impl TracesQuery {
    /// Create a new traces query
    pub fn new(time_range: TimeRange) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            time_range,
            filters: Vec::new(),
            trace_id: None,
            span_id: None,
            service_name: None,
            operation_name: None,
            limit: None,
            offset: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the trace ID filter
    pub fn with_trace_id(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    /// Set the span ID filter
    pub fn with_span_id(mut self, span_id: String) -> Self {
        self.span_id = Some(span_id);
        self
    }

    /// Set the service name filter
    pub fn with_service_name(mut self, service_name: String) -> Self {
        self.service_name = Some(service_name);
        self
    }

    /// Set the operation name filter
    pub fn with_operation_name(mut self, operation_name: String) -> Self {
        self.operation_name = Some(operation_name);
        self
    }

    /// Add a filter to the query
    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Set the query limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the query offset
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Add metadata to the query
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

impl LogsQuery {
    /// Create a new logs query
    pub fn new(time_range: TimeRange) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            time_range,
            filters: Vec::new(),
            log_level: None,
            service_name: None,
            limit: None,
            offset: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the log level filter
    pub fn with_log_level(mut self, log_level: String) -> Self {
        self.log_level = Some(log_level);
        self
    }

    /// Set the service name filter
    pub fn with_service_name(mut self, service_name: String) -> Self {
        self.service_name = Some(service_name);
        self
    }

    /// Add a filter to the query
    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Set the query limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the query offset
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Add metadata to the query
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

impl TelemetryQuery {
    /// Create a new telemetry query
    pub fn new(query_type: TelemetryQueryType, time_range: TimeRange) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            query_type,
            time_range,
            filters: Vec::new(),
            aggregations: Vec::new(),
            limit: None,
            offset: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a filter to the query
    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Add an aggregation to the query
    pub fn with_aggregation(mut self, aggregation: Aggregation) -> Self {
        self.aggregations.push(aggregation);
        self
    }

    /// Set the query limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the query offset
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Add metadata to the query
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::queries::time_range::TimeRange;

    #[test]
    fn test_metrics_query_creation() {
        let time_range = TimeRange::last_hours(1);
        let query = MetricsQuery::new(time_range);
        assert_eq!(query.filters.len(), 0);
        assert_eq!(query.aggregations.len(), 0);
    }

    #[test]
    fn test_traces_query_creation() {
        let time_range = TimeRange::last_hours(1);
        let query = TracesQuery::new(time_range);
        assert_eq!(query.filters.len(), 0);
        assert!(query.trace_id.is_none());
    }

    #[test]
    fn test_logs_query_creation() {
        let time_range = TimeRange::last_hours(1);
        let query = LogsQuery::new(time_range);
        assert_eq!(query.filters.len(), 0);
        assert!(query.log_level.is_none());
    }

    #[test]
    fn test_telemetry_query_creation() {
        let time_range = TimeRange::last_hours(1);
        let query = TelemetryQuery::new(TelemetryQueryType::Metrics, time_range);
        assert_eq!(query.filters.len(), 0);
        assert!(matches!(query.query_type, TelemetryQueryType::Metrics));
    }
}
