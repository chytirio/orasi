//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Time series analyzer
//!
//! This module provides time series analysis capabilities.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{TimeSeriesData, TimeSeriesPoint};

/// Time series analyzer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesAnalyzerConfig {
    /// Analyzer name
    pub name: String,

    /// Analyzer version
    pub version: String,

    /// Enable analysis
    pub enable_analysis: bool,

    /// Default window size in seconds
    pub default_window_secs: u64,

    /// Enable trend analysis
    pub enable_trend_analysis: bool,

    /// Enable seasonality analysis
    pub enable_seasonality_analysis: bool,

    /// Enable statistical analysis
    pub enable_statistical_analysis: bool,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl Default for TimeSeriesAnalyzerConfig {
    fn default() -> Self {
        Self {
            name: "time_series_analyzer".to_string(),
            version: "1.0.0".to_string(),
            enable_analysis: true,
            default_window_secs: 3600, // 1 hour
            enable_trend_analysis: true,
            enable_seasonality_analysis: true,
            enable_statistical_analysis: true,
            additional_config: HashMap::new(),
        }
    }
}

/// Time series analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesResult {
    /// Analysis ID
    pub id: Uuid,

    /// Series ID
    pub series_id: Uuid,

    /// Analysis timestamp
    pub timestamp: DateTime<Utc>,

    /// Statistical summary
    pub statistics: StatisticalSummary,

    /// Trend analysis
    pub trend_analysis: Option<TrendAnalysis>,

    /// Seasonality analysis
    pub seasonality_analysis: Option<SeasonalityAnalysis>,

    /// Analysis metadata
    pub metadata: HashMap<String, String>,
}

/// Statistical summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalSummary {
    /// Number of data points
    pub count: usize,

    /// Mean value
    pub mean: f64,

    /// Standard deviation
    pub std_dev: f64,

    /// Minimum value
    pub min: f64,

    /// Maximum value
    pub max: f64,

    /// Median value
    pub median: f64,

    /// First quartile
    pub q1: f64,

    /// Third quartile
    pub q3: f64,

    /// Skewness
    pub skewness: f64,

    /// Kurtosis
    pub kurtosis: f64,
}

/// Trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// Trend direction
    pub direction: TrendDirection,

    /// Trend slope
    pub slope: f64,

    /// Trend strength (R-squared)
    pub strength: f64,

    /// Trend confidence
    pub confidence: f64,

    /// Trend description
    pub description: String,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    /// Increasing trend
    Increasing,

    /// Decreasing trend
    Decreasing,

    /// No significant trend
    Stable,
}

/// Seasonality analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalityAnalysis {
    /// Seasonal period in seconds
    pub period_secs: u64,

    /// Seasonal strength
    pub strength: f64,

    /// Seasonal pattern
    pub pattern: Vec<f64>,

    /// Seasonality confidence
    pub confidence: f64,

    /// Seasonality description
    pub description: String,
}

/// Time series analyzer implementation
pub struct TimeSeriesAnalyzer {
    config: TimeSeriesAnalyzerConfig,
}

impl TimeSeriesAnalyzer {
    /// Create a new time series analyzer
    pub fn new(config: TimeSeriesAnalyzerConfig) -> Self {
        Self { config }
    }

    /// Calculate statistical summary
    fn calculate_statistics(&self, points: &[TimeSeriesPoint]) -> StatisticalSummary {
        if points.is_empty() {
            return StatisticalSummary {
                count: 0,
                mean: 0.0,
                std_dev: 0.0,
                min: 0.0,
                max: 0.0,
                median: 0.0,
                q1: 0.0,
                q3: 0.0,
                skewness: 0.0,
                kurtosis: 0.0,
            };
        }

        let values: Vec<f64> = points.iter().map(|p| p.value).collect();
        let count = values.len();
        let sum: f64 = values.iter().sum();
        let mean = sum / count as f64;

        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (count - 1) as f64;
        let std_dev = variance.sqrt();

        let mut sorted_values = values.clone();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let min = sorted_values[0];
        let max = sorted_values[count - 1];
        let median = if count % 2 == 0 {
            (sorted_values[count / 2 - 1] + sorted_values[count / 2]) / 2.0
        } else {
            sorted_values[count / 2]
        };

        let q1_idx = count / 4;
        let q3_idx = 3 * count / 4;
        let q1 = sorted_values[q1_idx];
        let q3 = sorted_values[q3_idx];

        // Calculate skewness
        let skewness = values
            .iter()
            .map(|v| ((v - mean) / std_dev).powi(3))
            .sum::<f64>()
            / count as f64;

        // Calculate kurtosis
        let kurtosis = values
            .iter()
            .map(|v| ((v - mean) / std_dev).powi(4))
            .sum::<f64>()
            / count as f64
            - 3.0;

        StatisticalSummary {
            count,
            mean,
            std_dev,
            min,
            max,
            median,
            q1,
            q3,
            skewness,
            kurtosis,
        }
    }

    /// Analyze trend
    fn analyze_trend(&self, points: &[TimeSeriesPoint]) -> Option<TrendAnalysis> {
        if points.len() < 2 {
            return None;
        }

        // Simple linear regression
        let n = points.len() as f64;
        let x_values: Vec<f64> = points.iter().enumerate().map(|(i, _)| i as f64).collect();
        let y_values: Vec<f64> = points.iter().map(|p| p.value).collect();

        let sum_x: f64 = x_values.iter().sum();
        let sum_y: f64 = y_values.iter().sum();
        let sum_xy: f64 = x_values
            .iter()
            .zip(y_values.iter())
            .map(|(x, y)| x * y)
            .sum();
        let sum_x2: f64 = x_values.iter().map(|x| x * x).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        // Calculate R-squared
        let y_mean = sum_y / n;
        let ss_tot: f64 = y_values.iter().map(|y| (y - y_mean).powi(2)).sum();
        let ss_res: f64 = x_values
            .iter()
            .zip(y_values.iter())
            .map(|(x, y)| (y - (slope * x + intercept)).powi(2))
            .sum();
        let r_squared = 1.0 - (ss_res / ss_tot);

        let direction = if slope > 0.01 {
            TrendDirection::Increasing
        } else if slope < -0.01 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };

        let description = match direction {
            TrendDirection::Increasing => format!("Increasing trend with slope {:.4}", slope),
            TrendDirection::Decreasing => format!("Decreasing trend with slope {:.4}", slope),
            TrendDirection::Stable => "No significant trend detected".to_string(),
        };

        Some(TrendAnalysis {
            direction,
            slope,
            strength: r_squared,
            confidence: r_squared.min(1.0),
            description,
        })
    }

    /// Analyze seasonality
    fn analyze_seasonality(&self, points: &[TimeSeriesPoint]) -> Option<SeasonalityAnalysis> {
        if points.len() < 10 {
            return None;
        }

        // Simple seasonality detection using autocorrelation
        let values: Vec<f64> = points.iter().map(|p| p.value).collect();
        let n = values.len();

        // Calculate autocorrelation for different lags
        let max_lag = (n / 2).min(100);
        let mut autocorr = Vec::new();

        for lag in 1..=max_lag {
            let mut sum = 0.0;
            let mut count = 0;

            for i in 0..(n - lag) {
                sum += values[i] * values[i + lag];
                count += 1;
            }

            if count > 0 {
                autocorr.push(sum / count as f64);
            }
        }

        // Find the lag with maximum autocorrelation
        if let Some((max_lag_idx, _)) = autocorr
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        {
            let period_secs = (max_lag_idx + 1) as u64 * self.config.default_window_secs;
            let strength = autocorr[max_lag_idx];

            if strength > 0.3 {
                let description = format!(
                    "Seasonal pattern detected with period {} seconds",
                    period_secs
                );

                return Some(SeasonalityAnalysis {
                    period_secs,
                    strength,
                    pattern: autocorr.clone(),
                    confidence: strength.min(1.0),
                    description,
                });
            }
        }

        None
    }
}
