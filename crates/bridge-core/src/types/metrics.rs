//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Metric data structures for the OpenTelemetry Data Lake Bridge
//!
//! This module provides metric-specific data structures including metric data,
//! types, values, and batches used throughout the bridge.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Metric data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricData {
    /// Metric name
    pub name: String,

    /// Metric description
    pub description: Option<String>,

    /// Metric unit
    pub unit: Option<String>,

    /// Metric type
    pub metric_type: MetricType,

    /// Metric value
    pub value: MetricValue,

    /// Metric labels
    pub labels: HashMap<String, String>,

    /// Metric timestamp
    pub timestamp: DateTime<Utc>,
}

/// Metric types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Metric values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    /// Counter value
    Counter(f64),

    /// Gauge value
    Gauge(f64),

    /// Histogram buckets and sum
    Histogram {
        buckets: Vec<HistogramBucket>,
        sum: f64,
        count: u64,
    },

    /// Summary quantiles and sum
    Summary {
        quantiles: Vec<SummaryQuantile>,
        sum: f64,
        count: u64,
    },
}

/// Histogram bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    /// Bucket upper bound
    pub upper_bound: f64,

    /// Bucket count
    pub count: u64,
}

/// Summary quantile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryQuantile {
    /// Quantile value (0.0 to 1.0)
    pub quantile: f64,

    /// Quantile value
    pub value: f64,
}

/// Metrics batch for lakehouse operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsBatch {
    /// Batch ID
    pub id: Uuid,

    /// Batch timestamp
    pub timestamp: DateTime<Utc>,

    /// Metrics data
    pub metrics: Vec<MetricData>,

    /// Batch metadata
    pub metadata: HashMap<String, String>,
}
