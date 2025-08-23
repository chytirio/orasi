//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Advanced analytics for time series analysis and machine learning
//!
//! This module provides advanced analytics capabilities including
//! time series analysis, anomaly detection, and machine learning integration.

pub mod anomaly_detection;
pub mod clustering;
pub mod forecasting;
pub mod ml_integration;
pub mod time_series;

// Re-export main types
pub use anomaly_detection::{AnomalyDetector, AnomalyDetectorConfig};
pub use clustering::{ClusterAnalyzer, ClusterAnalyzerConfig};
pub use forecasting::{ForecastModel, ForecastResult, Forecaster, ForecasterConfig};
pub use ml_integration::{MLModel, MLModelConfig, MLPredictor, MLPredictorConfig};
pub use time_series::{TimeSeriesAnalyzer, TimeSeriesAnalyzerConfig, TimeSeriesResult};

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Analytics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    /// Enable time series analysis
    pub enable_time_series: bool,

    /// Enable anomaly detection
    pub enable_anomaly_detection: bool,

    /// Enable machine learning integration
    pub enable_ml_integration: bool,

    /// Enable forecasting
    pub enable_forecasting: bool,

    /// Enable clustering
    pub enable_clustering: bool,

    /// Default analysis window in seconds
    pub default_analysis_window_secs: u64,

    /// Confidence threshold for predictions
    pub confidence_threshold: f64,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            enable_time_series: true,
            enable_anomaly_detection: true,
            enable_ml_integration: true,
            enable_forecasting: true,
            enable_clustering: true,
            default_analysis_window_secs: 3600, // 1 hour
            confidence_threshold: 0.8,
            additional_config: HashMap::new(),
        }
    }
}

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Value
    pub value: f64,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Time series data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesData {
    /// Series ID
    pub id: Uuid,

    /// Series name
    pub name: String,

    /// Data points
    pub points: Vec<TimeSeriesPoint>,

    /// Series metadata
    pub metadata: HashMap<String, String>,
}

/// Anomaly type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnomalyType {
    /// Point anomaly - single data point is anomalous
    Point,

    /// Contextual anomaly - data point is anomalous in context
    Contextual,

    /// Collective anomaly - group of data points is anomalous
    Collective,

    /// Trend anomaly - trend change is anomalous
    Trend,

    /// Seasonal anomaly - seasonal pattern is anomalous
    Seasonal,
}

/// Anomaly result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyResult {
    /// Anomaly ID
    pub id: Uuid,

    /// Anomaly type
    pub anomaly_type: AnomalyType,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Confidence score
    pub confidence: f64,

    /// Severity score
    pub severity: f64,

    /// Description
    pub description: String,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    /// Prediction ID
    pub id: Uuid,

    /// Predicted value
    pub predicted_value: f64,

    /// Confidence interval lower bound
    pub confidence_lower: f64,

    /// Confidence interval upper bound
    pub confidence_upper: f64,

    /// Confidence score
    pub confidence: f64,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Model used
    pub model_name: String,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Cluster type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClusterType {
    /// K-means clustering
    KMeans,

    /// DBSCAN clustering
    DBSCAN,

    /// Hierarchical clustering
    Hierarchical,

    /// Spectral clustering
    Spectral,
}

/// Cluster result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterResult {
    /// Cluster ID
    pub id: Uuid,

    /// Cluster type
    pub cluster_type: ClusterType,

    /// Cluster label
    pub label: i32,

    /// Cluster center
    pub center: Vec<f64>,

    /// Cluster size
    pub size: usize,

    /// Cluster score
    pub score: f64,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Analytics trait
#[async_trait]
pub trait Analytics: Send + Sync {
    /// Analyze time series data
    async fn analyze_time_series(&self, data: &TimeSeriesData) -> BridgeResult<TimeSeriesResult>;

    /// Detect anomalies
    async fn detect_anomalies(&self, data: &TimeSeriesData) -> BridgeResult<Vec<AnomalyResult>>;

    /// Make predictions
    async fn make_predictions(
        &self,
        data: &TimeSeriesData,
        horizon: u64,
    ) -> BridgeResult<Vec<PredictionResult>>;

    /// Perform clustering
    async fn perform_clustering(&self, data: &TimeSeriesData) -> BridgeResult<Vec<ClusterResult>>;
}
