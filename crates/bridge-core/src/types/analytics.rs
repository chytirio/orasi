//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Analytics types for the OpenTelemetry Data Lake Bridge
//!
//! This module provides analytics-related data structures for IDE plugin
//! integration and analytics functionality.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::{Aggregation, Filter, TimeRange};

/// Analytics types for IDE plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalyticsType {
    /// Development workflow analytics
    WorkflowAnalytics,

    /// Agent performance analytics
    AgentAnalytics,

    /// Multi-repo coordination analytics
    MultiRepoAnalytics,

    /// Real-time alerting
    RealTimeAlerting,

    /// Interactive querying
    InteractiveQuerying,

    /// Data visualization
    DataVisualization,

    /// Custom analytics
    Custom(String),
}

/// Analytics query for IDE plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsQuery {
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

    /// Query metadata
    pub metadata: HashMap<String, String>,
}

/// Analytics request for IDE plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsRequest {
    /// Request ID
    pub id: Uuid,

    /// Request timestamp
    pub timestamp: DateTime<Utc>,

    /// Analytics type
    pub query_type: AnalyticsType,

    /// Request parameters
    pub parameters: HashMap<String, serde_json::Value>,

    /// Output format
    pub output_format: OutputFormat,

    /// Request metadata
    pub metadata: HashMap<String, String>,
}

/// Output formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Json,
    Csv,
    Parquet,
    Arrow,
    Custom(String),
}

/// Analytics response for IDE plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsResponse {
    /// Request ID
    pub request_id: Uuid,

    /// Response timestamp
    pub timestamp: DateTime<Utc>,

    /// Response status
    pub status: AnalyticsStatus,

    /// Response data
    pub data: serde_json::Value,

    /// Response metadata
    pub metadata: HashMap<String, String>,

    /// Response errors
    pub errors: Vec<AnalyticsError>,
}

/// Analytics status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalyticsStatus {
    Success,
    Partial,
    Failed,
    Timeout,
}

/// Analytics error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Error details
    pub details: Option<serde_json::Value>,
}
