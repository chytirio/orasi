//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Query result types for the OpenTelemetry Data Lake Bridge

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::types::{MetricData, TraceData, LogData};

/// Query results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResult {
    /// Query ID
    pub query_id: Uuid,

    /// Result timestamp
    pub timestamp: DateTime<Utc>,

    /// Result status
    pub status: QueryStatus,

    /// Result data
    pub data: Vec<MetricData>,

    /// Result metadata
    pub metadata: HashMap<String, String>,

    /// Query duration in milliseconds
    pub duration_ms: u64,

    /// Result errors
    pub errors: Vec<QueryError>,
}

/// Traces result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracesResult {
    /// Query ID
    pub query_id: Uuid,

    /// Result timestamp
    pub timestamp: DateTime<Utc>,

    /// Result status
    pub status: QueryStatus,

    /// Result data
    pub data: Vec<TraceData>,

    /// Result metadata
    pub metadata: HashMap<String, String>,

    /// Query duration in milliseconds
    pub duration_ms: u64,

    /// Result errors
    pub errors: Vec<QueryError>,
}

/// Logs result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsResult {
    /// Query ID
    pub query_id: Uuid,

    /// Result timestamp
    pub timestamp: DateTime<Utc>,

    /// Result status
    pub status: QueryStatus,

    /// Result data
    pub data: Vec<LogData>,

    /// Result metadata
    pub metadata: HashMap<String, String>,

    /// Query duration in milliseconds
    pub duration_ms: u64,

    /// Result errors
    pub errors: Vec<QueryError>,
}

/// Query status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryStatus {
    Success,
    Partial,
    Failed,
    Timeout,
}

/// Query error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Error details
    pub details: Option<serde_json::Value>,
}

impl MetricsResult {
    /// Create a new metrics result
    pub fn new(query_id: Uuid, data: Vec<MetricData>) -> Self {
        Self {
            query_id,
            timestamp: Utc::now(),
            status: QueryStatus::Success,
            data,
            metadata: HashMap::new(),
            duration_ms: 0,
            errors: Vec::new(),
        }
    }

    /// Set the result status
    pub fn with_status(mut self, status: QueryStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the query duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Add metadata to the result
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add an error to the result
    pub fn with_error(mut self, error: QueryError) -> Self {
        self.errors.push(error);
        self
    }
}

impl TracesResult {
    /// Create a new traces result
    pub fn new(query_id: Uuid, data: Vec<TraceData>) -> Self {
        Self {
            query_id,
            timestamp: Utc::now(),
            status: QueryStatus::Success,
            data,
            metadata: HashMap::new(),
            duration_ms: 0,
            errors: Vec::new(),
        }
    }

    /// Set the result status
    pub fn with_status(mut self, status: QueryStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the query duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Add metadata to the result
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add an error to the result
    pub fn with_error(mut self, error: QueryError) -> Self {
        self.errors.push(error);
        self
    }
}

impl LogsResult {
    /// Create a new logs result
    pub fn new(query_id: Uuid, data: Vec<LogData>) -> Self {
        Self {
            query_id,
            timestamp: Utc::now(),
            status: QueryStatus::Success,
            data,
            metadata: HashMap::new(),
            duration_ms: 0,
            errors: Vec::new(),
        }
    }

    /// Set the result status
    pub fn with_status(mut self, status: QueryStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the query duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Add metadata to the result
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add an error to the result
    pub fn with_error(mut self, error: QueryError) -> Self {
        self.errors.push(error);
        self
    }
}

impl QueryError {
    /// Create a new query error
    pub fn new(code: String, message: String) -> Self {
        Self {
            code,
            message,
            details: None,
        }
    }

    /// Add details to the error
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_result_creation() {
        let query_id = Uuid::new_v4();
        let data = vec![];
        let result = MetricsResult::new(query_id, data);
        assert_eq!(result.query_id, query_id);
        assert!(matches!(result.status, QueryStatus::Success));
    }

    #[test]
    fn test_traces_result_creation() {
        let query_id = Uuid::new_v4();
        let data = vec![];
        let result = TracesResult::new(query_id, data);
        assert_eq!(result.query_id, query_id);
        assert!(matches!(result.status, QueryStatus::Success));
    }

    #[test]
    fn test_logs_result_creation() {
        let query_id = Uuid::new_v4();
        let data = vec![];
        let result = LogsResult::new(query_id, data);
        assert_eq!(result.query_id, query_id);
        assert!(matches!(result.status, QueryStatus::Success));
    }

    #[test]
    fn test_query_error_creation() {
        let error = QueryError::new("TEST_ERROR".to_string(), "Test error message".to_string());
        assert_eq!(error.code, "TEST_ERROR");
        assert_eq!(error.message, "Test error message");
    }
}
