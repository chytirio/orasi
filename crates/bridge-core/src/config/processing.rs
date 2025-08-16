//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Processing configuration for the OpenTelemetry Data Lake Bridge
//!
//! This module provides configuration structures for data processing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

/// Processing configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ProcessingConfig {
    /// Number of worker threads for processing
    #[validate(range(min = 1, max = 64))]
    pub worker_threads: usize,

    /// Enable real-time stream processing
    pub enable_streaming: bool,

    /// Stream processing window size in milliseconds
    #[validate(range(min = 1000, max = 300000))]
    pub stream_window_ms: u64,

    /// Enable data transformation
    pub enable_transformation: bool,

    /// Transformation rules configuration
    pub transformation_rules: Option<Vec<TransformationRule>>,

    /// Enable data filtering
    pub enable_filtering: bool,

    /// Filter rules configuration
    pub filter_rules: Option<Vec<FilterRule>>,

    /// Enable data aggregation
    pub enable_aggregation: bool,

    /// Aggregation rules configuration
    pub aggregation_rules: Option<Vec<AggregationRule>>,

    /// Enable anomaly detection
    pub enable_anomaly_detection: bool,

    /// Anomaly detection configuration
    pub anomaly_detection: Option<AnomalyDetectionConfig>,

    /// Query timeout in seconds
    #[validate(range(min = 1, max = 3600))]
    pub query_timeout_secs: u64,

    /// Maximum query memory in bytes
    pub max_query_memory: Option<u64>,

    /// Enable query caching
    pub enable_query_caching: bool,

    /// Cache size in bytes
    pub cache_size: Option<u64>,

    /// Cache TTL in seconds
    pub cache_ttl_secs: Option<u64>,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            enable_streaming: true,
            stream_window_ms: 60000,
            enable_transformation: false,
            transformation_rules: None,
            enable_filtering: false,
            filter_rules: None,
            enable_aggregation: false,
            aggregation_rules: None,
            enable_anomaly_detection: false,
            anomaly_detection: None,
            query_timeout_secs: 30,
            max_query_memory: None,
            enable_query_caching: true,
            cache_size: Some(1024 * 1024 * 100), // 100MB
            cache_ttl_secs: Some(300),           // 5 minutes
        }
    }
}

/// Transformation rule
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TransformationRule {
    /// Rule name
    pub name: String,

    /// Rule description
    pub description: Option<String>,

    /// Source field
    pub source_field: String,

    /// Target field
    pub target_field: String,

    /// Transformation type
    pub transformation_type: TransformationType,

    /// Transformation parameters
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

/// Transformation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformationType {
    Copy,
    Rename,
    Extract,
    Concatenate,
    Split,
    Format,
    Parse,
    Convert,
    Custom,
}

/// Filter rule
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct FilterRule {
    /// Rule name
    pub name: String,

    /// Rule description
    pub description: Option<String>,

    /// Field to filter on
    pub field: String,

    /// Filter operator
    pub operator: FilterOperator,

    /// Filter value
    pub value: serde_json::Value,

    /// Filter action (include, exclude)
    pub action: FilterAction,
}

/// Filter operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    Regex,
    In,
    NotIn,
}

/// Filter actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterAction {
    Include,
    Exclude,
}

/// Aggregation rule
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AggregationRule {
    /// Rule name
    pub name: String,

    /// Rule description
    pub description: Option<String>,

    /// Source field
    pub source_field: String,

    /// Target field
    pub target_field: String,

    /// Aggregation function
    pub function: AggregationFunction,

    /// Time window in milliseconds
    pub time_window_ms: Option<u64>,

    /// Group by fields
    pub group_by: Option<Vec<String>>,
}

/// Aggregation functions
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
    Custom,
}

/// Anomaly detection configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AnomalyDetectionConfig {
    /// Enable statistical anomaly detection
    pub enable_statistical: bool,

    /// Statistical threshold (standard deviations)
    #[validate(range(min = 1.0, max = 10.0))]
    pub statistical_threshold: f64,

    /// Enable machine learning anomaly detection
    pub enable_ml: bool,

    /// ML model path
    pub ml_model_path: Option<std::path::PathBuf>,

    /// ML model parameters
    pub ml_parameters: Option<HashMap<String, serde_json::Value>>,

    /// Alerting configuration
    pub alerting: AnomalyAlertingConfig,
}

/// Anomaly alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AnomalyAlertingConfig {
    /// Enable email alerts
    pub enable_email_alerts: bool,

    /// Email recipients
    pub email_recipients: Option<Vec<String>>,

    /// Enable webhook alerts
    pub enable_webhook_alerts: bool,

    /// Webhook URL
    pub webhook_url: Option<String>,

    /// Alert threshold
    #[validate(range(min = 1, max = 100))]
    pub alert_threshold: u32,
}
