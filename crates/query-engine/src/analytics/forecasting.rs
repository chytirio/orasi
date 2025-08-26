//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Forecasting for time series data
//!
//! This module provides forecasting capabilities for time series data.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::{PredictionResult, TimeSeriesData, TimeSeriesPoint};

/// Forecaster configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecasterConfig {
    /// Forecaster name
    pub name: String,

    /// Forecaster version
    pub version: String,

    /// Enable forecasting
    pub enable_forecasting: bool,

    /// Forecast horizon
    pub forecast_horizon: u64,

    /// Forecasting algorithm to use
    pub algorithm: ForecastingAlgorithm,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl Default for ForecasterConfig {
    fn default() -> Self {
        Self {
            name: "default_forecaster".to_string(),
            version: "1.0.0".to_string(),
            enable_forecasting: true,
            forecast_horizon: 24, // 24 time units ahead
            algorithm: ForecastingAlgorithm::default(),
            additional_config: HashMap::new(),
        }
    }
}

/// Forecasting algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForecastingAlgorithm {
    /// Simple Moving Average
    SimpleMovingAverage,
    /// Linear Regression
    LinearRegression,
    /// Exponential Smoothing
    ExponentialSmoothing,
    /// ARIMA (AutoRegressive Integrated Moving Average)
    ARIMA,
    /// Prophet-like decomposition
    Prophet,
    /// Ensemble (combines multiple methods)
    Ensemble,
}

impl Default for ForecastingAlgorithm {
    fn default() -> Self {
        Self::LinearRegression
    }
}

/// Forecast result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastResult {
    /// Forecast ID
    pub id: String,

    /// Forecast name
    pub name: String,

    /// Forecast description
    pub description: String,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Forecast model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastModel {
    /// Model ID
    pub id: String,

    /// Model name
    pub name: String,

    /// Model description
    pub description: String,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// Forecaster trait
#[async_trait]
pub trait Forecaster: Send + Sync {
    /// Generate forecast
    async fn forecast(
        &self,
        data: &TimeSeriesData,
        horizon: u64,
    ) -> BridgeResult<Vec<PredictionResult>>;
}

/// Default forecaster implementation
pub struct DefaultForecaster {
    config: ForecasterConfig,
}

impl DefaultForecaster {
    /// Create a new forecaster
    pub fn new(config: ForecasterConfig) -> Self {
        Self { config }
    }

    /// Get the algorithm to use
    fn get_algorithm(&self) -> &ForecastingAlgorithm {
        &self.config.algorithm
    }
}

#[async_trait]
impl Forecaster for DefaultForecaster {
    async fn forecast(
        &self,
        data: &TimeSeriesData,
        horizon: u64,
    ) -> BridgeResult<Vec<PredictionResult>> {
        info!("Generating forecast for series '{}' with horizon {}", data.name, horizon);

        // Validate input data
        if data.points.len() < 2 {
            return Err(bridge_core::BridgeError::internal(
                "Insufficient data points for forecasting (minimum 2 required)",
            ));
        }

        // Sort points by timestamp to ensure chronological order
        let mut sorted_points = data.points.clone();
        sorted_points.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // Extract values and timestamps
        let values: Vec<f64> = sorted_points.iter().map(|p| p.value).collect();
        let timestamps: Vec<DateTime<Utc>> = sorted_points.iter().map(|p| p.timestamp).collect();

        // Generate forecast based on selected algorithm
        let predictions = match self.get_algorithm() {
            ForecastingAlgorithm::SimpleMovingAverage => {
                self.simple_moving_average_forecast(&values, &timestamps, horizon).await?
            }
            ForecastingAlgorithm::LinearRegression => {
                self.linear_regression_forecast(&values, &timestamps, horizon).await?
            }
            ForecastingAlgorithm::ExponentialSmoothing => {
                self.exponential_smoothing_forecast(&values, &timestamps, horizon).await?
            }
            ForecastingAlgorithm::ARIMA => {
                self.arima_forecast(&values, &timestamps, horizon).await?
            }
            ForecastingAlgorithm::Prophet => {
                self.prophet_forecast(&values, &timestamps, horizon).await?
            }
            ForecastingAlgorithm::Ensemble => {
                self.ensemble_forecast(&values, &timestamps, horizon).await?
            }
        };

        debug!("Generated {} forecast predictions", predictions.len());
        Ok(predictions)
    }
}

impl DefaultForecaster {
    /// Simple Moving Average forecasting
    async fn simple_moving_average_forecast(
        &self,
        values: &[f64],
        timestamps: &[DateTime<Utc>],
        horizon: u64,
    ) -> BridgeResult<Vec<PredictionResult>> {
        let window_size = (values.len() / 4).max(3).min(20); // Adaptive window size
        let moving_avg = self.calculate_moving_average(values, window_size);
        
        let last_avg = moving_avg.last().unwrap_or(&values[values.len() - 1]);
        let last_timestamp = timestamps[timestamps.len() - 1];
        
        let predictions: Vec<PredictionResult> = (1..=horizon as usize)
            .map(|i| {
                let forecast_timestamp = last_timestamp + Duration::seconds(i as i64);
                let predicted_value = *last_avg;
                
                PredictionResult {
                    id: Uuid::new_v4(),
                    predicted_value,
                    confidence_lower: predicted_value * 0.9,
                    confidence_upper: predicted_value * 1.1,
                    confidence: 0.7,
                    timestamp: forecast_timestamp,
                    model_name: "SimpleMovingAverage".to_string(),
                    metadata: HashMap::from([
                        ("window_size".to_string(), window_size.to_string()),
                        ("algorithm".to_string(), "simple_moving_average".to_string()),
                    ]),
                }
            })
            .collect();

        Ok(predictions)
    }

    /// Linear Regression forecasting
    async fn linear_regression_forecast(
        &self,
        values: &[f64],
        timestamps: &[DateTime<Utc>],
        horizon: u64,
    ) -> BridgeResult<Vec<PredictionResult>> {
        let (slope, intercept, r_squared) = self.calculate_linear_regression(values);
        let last_timestamp = timestamps[timestamps.len() - 1];
        
        let predictions: Vec<PredictionResult> = (1..=horizon as usize)
            .map(|i| {
                let forecast_timestamp = last_timestamp + Duration::seconds(i as i64);
                let time_index = (timestamps.len() + i) as f64;
                let predicted_value = slope * time_index + intercept;
                
                // Calculate confidence interval based on R-squared
                let confidence_width = (1.0 - r_squared).max(0.1) * predicted_value.abs() * 0.2;
                
                PredictionResult {
                    id: Uuid::new_v4(),
                    predicted_value,
                    confidence_lower: predicted_value - confidence_width,
                    confidence_upper: predicted_value + confidence_width,
                    confidence: r_squared.max(0.5),
                    timestamp: forecast_timestamp,
                    model_name: "LinearRegression".to_string(),
                    metadata: HashMap::from([
                        ("slope".to_string(), slope.to_string()),
                        ("intercept".to_string(), intercept.to_string()),
                        ("r_squared".to_string(), r_squared.to_string()),
                        ("algorithm".to_string(), "linear_regression".to_string()),
                    ]),
                }
            })
            .collect();

        Ok(predictions)
    }

    /// Exponential Smoothing forecasting
    async fn exponential_smoothing_forecast(
        &self,
        values: &[f64],
        timestamps: &[DateTime<Utc>],
        horizon: u64,
    ) -> BridgeResult<Vec<PredictionResult>> {
        let alpha = 0.3; // Smoothing factor
        let smoothed_values = self.calculate_exponential_smoothing(values, alpha);
        
        let last_smoothed = smoothed_values.last().unwrap_or(&values[values.len() - 1]);
        let last_timestamp = timestamps[timestamps.len() - 1];
        
        let predictions: Vec<PredictionResult> = (1..=horizon as usize)
            .map(|i| {
                let forecast_timestamp = last_timestamp + Duration::seconds(i as i64);
                let predicted_value = *last_smoothed;
                
                PredictionResult {
                    id: Uuid::new_v4(),
                    predicted_value,
                    confidence_lower: predicted_value * 0.85,
                    confidence_upper: predicted_value * 1.15,
                    confidence: 0.75,
                    timestamp: forecast_timestamp,
                    model_name: "ExponentialSmoothing".to_string(),
                    metadata: HashMap::from([
                        ("alpha".to_string(), alpha.to_string()),
                        ("algorithm".to_string(), "exponential_smoothing".to_string()),
                    ]),
                }
            })
            .collect();

        Ok(predictions)
    }

    /// ARIMA forecasting (simplified implementation)
    async fn arima_forecast(
        &self,
        values: &[f64],
        timestamps: &[DateTime<Utc>],
        horizon: u64,
    ) -> BridgeResult<Vec<PredictionResult>> {
        // Simplified ARIMA implementation
        // In a real implementation, you would use a proper ARIMA library
        
        let (trend, seasonal) = self.decompose_time_series(values);
        let last_timestamp = timestamps[timestamps.len() - 1];
        
        let predictions: Vec<PredictionResult> = (1..=horizon as usize)
            .map(|i| {
                let forecast_timestamp = last_timestamp + Duration::seconds(i as i64);
                
                // Combine trend and seasonal components
                let trend_component = trend.last().unwrap_or(&0.0) * i as f64;
                let seasonal_component = if seasonal.len() > 0 {
                    seasonal[i % seasonal.len()]
                } else {
                    0.0
                };
                
                let predicted_value = trend_component + seasonal_component;
                
                PredictionResult {
                    id: Uuid::new_v4(),
                    predicted_value,
                    confidence_lower: predicted_value * 0.8,
                    confidence_upper: predicted_value * 1.2,
                    confidence: 0.8,
                    timestamp: forecast_timestamp,
                    model_name: "ARIMA".to_string(),
                    metadata: HashMap::from([
                        ("trend_component".to_string(), trend_component.to_string()),
                        ("seasonal_component".to_string(), seasonal_component.to_string()),
                        ("algorithm".to_string(), "arima".to_string()),
                    ]),
                }
            })
            .collect();

        Ok(predictions)
    }

    /// Prophet-like forecasting with decomposition
    async fn prophet_forecast(
        &self,
        values: &[f64],
        timestamps: &[DateTime<Utc>],
        horizon: u64,
    ) -> BridgeResult<Vec<PredictionResult>> {
        // Prophet-like decomposition: trend + seasonal + residual
        let (trend, seasonal, _residual) = self.prophet_decompose(values);
        let last_timestamp = timestamps[timestamps.len() - 1];
        
        let predictions: Vec<PredictionResult> = (1..=horizon as usize)
            .map(|i| {
                let forecast_timestamp = last_timestamp + Duration::seconds(i as i64);
                
                let trend_component = trend.last().unwrap_or(&0.0) * i as f64;
                let seasonal_component = if seasonal.len() > 0 {
                    seasonal[i % seasonal.len()]
                } else {
                    0.0
                };
                
                let predicted_value = trend_component + seasonal_component;
                
                PredictionResult {
                    id: Uuid::new_v4(),
                    predicted_value,
                    confidence_lower: predicted_value * 0.85,
                    confidence_upper: predicted_value * 1.15,
                    confidence: 0.85,
                    timestamp: forecast_timestamp,
                    model_name: "Prophet".to_string(),
                    metadata: HashMap::from([
                        ("trend_component".to_string(), trend_component.to_string()),
                        ("seasonal_component".to_string(), seasonal_component.to_string()),
                        ("algorithm".to_string(), "prophet".to_string()),
                    ]),
                }
            })
            .collect();

        Ok(predictions)
    }

    /// Ensemble forecasting (combines multiple methods)
    async fn ensemble_forecast(
        &self,
        values: &[f64],
        timestamps: &[DateTime<Utc>],
        horizon: u64,
    ) -> BridgeResult<Vec<PredictionResult>> {
        // Generate forecasts using multiple methods
        let linear_forecast = self.linear_regression_forecast(values, timestamps, horizon).await?;
        let exp_forecast = self.exponential_smoothing_forecast(values, timestamps, horizon).await?;
        let sma_forecast = self.simple_moving_average_forecast(values, timestamps, horizon).await?;
        
        let last_timestamp = timestamps[timestamps.len() - 1];
        
        let predictions: Vec<PredictionResult> = (0..horizon as usize)
            .map(|i| {
                let forecast_timestamp = last_timestamp + Duration::seconds((i + 1) as i64);
                
                // Weighted average of predictions
                let linear_pred = linear_forecast[i].predicted_value;
                let exp_pred = exp_forecast[i].predicted_value;
                let sma_pred = sma_forecast[i].predicted_value;
                
                let predicted_value = linear_pred * 0.4 + exp_pred * 0.4 + sma_pred * 0.2;
                
                // Calculate ensemble confidence
                let predictions = vec![linear_pred, exp_pred, sma_pred];
                let variance = self.calculate_variance(&predictions);
                let confidence = (1.0 / (1.0 + variance)).max(0.6);
                
                PredictionResult {
                    id: Uuid::new_v4(),
                    predicted_value,
                    confidence_lower: predicted_value * 0.8,
                    confidence_upper: predicted_value * 1.2,
                    confidence,
                    timestamp: forecast_timestamp,
                    model_name: "Ensemble".to_string(),
                    metadata: HashMap::from([
                        ("linear_prediction".to_string(), linear_pred.to_string()),
                        ("exponential_prediction".to_string(), exp_pred.to_string()),
                        ("sma_prediction".to_string(), sma_pred.to_string()),
                        ("variance".to_string(), variance.to_string()),
                        ("algorithm".to_string(), "ensemble".to_string()),
                    ]),
                }
            })
            .collect();

        Ok(predictions)
    }

    // Helper methods for calculations

    /// Calculate moving average
    fn calculate_moving_average(&self, values: &[f64], window_size: usize) -> Vec<f64> {
        if values.len() < window_size {
            return values.to_vec();
        }

        values
            .windows(window_size)
            .map(|window| window.iter().sum::<f64>() / window_size as f64)
            .collect()
    }

    /// Calculate linear regression parameters
    fn calculate_linear_regression(&self, values: &[f64]) -> (f64, f64, f64) {
        let n = values.len() as f64;
        let x_values: Vec<f64> = (0..values.len()).map(|i| i as f64).collect();
        
        let sum_x: f64 = x_values.iter().sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = x_values.iter().zip(values.iter()).map(|(x, y)| x * y).sum();
        let sum_x2: f64 = x_values.iter().map(|x| x * x).sum();
        
        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;
        
        // Calculate R-squared
        let mean_y = sum_y / n;
        let ss_tot: f64 = values.iter().map(|y| (y - mean_y).powi(2)).sum();
        let ss_res: f64 = x_values.iter().zip(values.iter())
            .map(|(x, y)| (y - (slope * x + intercept)).powi(2))
            .sum();
        let r_squared = 1.0 - (ss_res / ss_tot);
        
        (slope, intercept, r_squared)
    }

    /// Calculate exponential smoothing
    fn calculate_exponential_smoothing(&self, values: &[f64], alpha: f64) -> Vec<f64> {
        if values.is_empty() {
            return vec![];
        }

        let mut smoothed = Vec::with_capacity(values.len());
        smoothed.push(values[0]); // First value is the same

        for i in 1..values.len() {
            let smoothed_value = alpha * values[i] + (1.0 - alpha) * smoothed[i - 1];
            smoothed.push(smoothed_value);
        }

        smoothed
    }

    /// Decompose time series into trend and seasonal components
    fn decompose_time_series(&self, values: &[f64]) -> (Vec<f64>, Vec<f64>) {
        if values.len() < 4 {
            return (values.to_vec(), vec![]);
        }

        // Simple trend calculation using moving average
        let trend_window = (values.len() / 4).max(2);
        let trend = self.calculate_moving_average(values, trend_window);

        // Seasonal component (simplified)
        let seasonal_period = (values.len() / 4).max(2);
        let mut seasonal = vec![0.0; seasonal_period];

        for i in 0..values.len() {
            let seasonal_idx = i % seasonal_period;
            seasonal[seasonal_idx] += values[i] - trend.get(i).unwrap_or(&values[i]);
        }

        // Average the seasonal components
        let count = (values.len() / seasonal_period) as f64;
        for val in &mut seasonal {
            *val /= count;
        }

        (trend, seasonal)
    }

    /// Prophet-like decomposition
    fn prophet_decompose(&self, values: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let (trend, seasonal) = self.decompose_time_series(values);
        
        let mut residual = Vec::with_capacity(values.len());
        for i in 0..values.len() {
            let trend_val = trend.get(i).unwrap_or(&values[i]);
            let seasonal_val = if seasonal.len() > 0 {
                seasonal[i % seasonal.len()]
            } else {
                0.0
            };
            residual.push(values[i] - trend_val - seasonal_val);
        }

        (trend, seasonal, residual)
    }

    /// Calculate variance
    fn calculate_variance(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / (values.len() - 1) as f64;

        variance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_forecasting_algorithms() {
        // Create sample time series data
        let mut points = Vec::new();
        let base_time = Utc::now();
        
        // Create a simple increasing trend
        for i in 0..10 {
            let timestamp = base_time + Duration::seconds(i as i64);
            let value = 100.0 + (i as f64 * 5.0); // Linear trend: 100, 105, 110, ...
            points.push(TimeSeriesPoint {
                timestamp,
                value,
                metadata: HashMap::new(),
            });
        }

        let data = TimeSeriesData {
            id: Uuid::new_v4(),
            name: "test_series".to_string(),
            points,
            metadata: HashMap::new(),
        };

        // Test each forecasting algorithm
        let algorithms = vec![
            ForecastingAlgorithm::SimpleMovingAverage,
            ForecastingAlgorithm::LinearRegression,
            ForecastingAlgorithm::ExponentialSmoothing,
            ForecastingAlgorithm::ARIMA,
            ForecastingAlgorithm::Prophet,
            ForecastingAlgorithm::Ensemble,
        ];

        for algorithm in algorithms {
            let config = ForecasterConfig {
                name: "test_forecaster".to_string(),
                version: "1.0.0".to_string(),
                enable_forecasting: true,
                forecast_horizon: 5,
                algorithm: algorithm.clone(),
                additional_config: HashMap::new(),
            };

            let forecaster = DefaultForecaster::new(config);
            let predictions = forecaster.forecast(&data, 3).await.unwrap();

            // Verify predictions
            assert_eq!(predictions.len(), 3, "Should generate 3 predictions for algorithm {:?}", algorithm);
            
            for prediction in &predictions {
                assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
                assert!(prediction.confidence_lower <= prediction.confidence_upper);
                assert!(!prediction.model_name.is_empty());
                assert!(!prediction.metadata.is_empty());
            }

            println!("✅ {:?} algorithm: Generated {} predictions", algorithm, predictions.len());
        }
    }

    #[test]
    fn test_linear_regression_calculation() {
        let forecaster = DefaultForecaster::new(ForecasterConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enable_forecasting: true,
            forecast_horizon: 5,
            algorithm: ForecastingAlgorithm::LinearRegression,
            additional_config: HashMap::new(),
        });

        // Test with simple linear data: y = 2x + 1
        let values = vec![1.0, 3.0, 5.0, 7.0, 9.0];
        let (slope, intercept, r_squared) = forecaster.calculate_linear_regression(&values);
        
        assert!((slope - 2.0).abs() < 0.01, "Expected slope ~2.0, got {}", slope);
        assert!((intercept - 1.0).abs() < 0.01, "Expected intercept ~1.0, got {}", intercept);
        assert!(r_squared > 0.99, "Expected high R-squared for perfect linear data, got {}", r_squared);
    }

    #[test]
    fn test_moving_average_calculation() {
        let forecaster = DefaultForecaster::new(ForecasterConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enable_forecasting: true,
            forecast_horizon: 5,
            algorithm: ForecastingAlgorithm::SimpleMovingAverage,
            additional_config: HashMap::new(),
        });

        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let moving_avg = forecaster.calculate_moving_average(&values, 3);
        
        // Expected: [1, 2, 3, 4, 5] with window 3
        // Moving averages: [1, 2, (1+2+3)/3=2, (2+3+4)/3=3, (3+4+5)/3=4]
        assert_eq!(moving_avg.len(), 3);
        assert!((moving_avg[0] - 2.0).abs() < 0.01);
        assert!((moving_avg[1] - 3.0).abs() < 0.01);
        assert!((moving_avg[2] - 4.0).abs() < 0.01);
    }
}
