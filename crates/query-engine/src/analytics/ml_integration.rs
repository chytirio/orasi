//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Machine learning integration
//!
//! This module provides machine learning integration capabilities.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{PredictionResult, TimeSeriesData};

/// ML model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLModelConfig {
    /// Model name
    pub name: String,

    /// Model version
    pub version: String,

    /// Model type
    pub model_type: String,

    /// Model path
    pub model_path: String,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// ML model trait
#[async_trait]
pub trait MLModel: Send + Sync {
    /// Load the model
    async fn load(&mut self) -> BridgeResult<()>;

    /// Make predictions
    async fn predict(&self, input: &[f64]) -> BridgeResult<Vec<f64>>;

    /// Get model info
    fn get_info(&self) -> MLModelConfig;
}

/// ML predictor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLPredictorConfig {
    /// Predictor name
    pub name: String,

    /// Predictor version
    pub version: String,

    /// Enable prediction
    pub enable_prediction: bool,

    /// Model configurations
    pub models: Vec<MLModelConfig>,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

/// ML predictor trait
#[async_trait]
pub trait MLPredictor: Send + Sync {
    /// Make predictions on time series data
    async fn predict(
        &self,
        data: &TimeSeriesData,
        horizon: u64,
    ) -> BridgeResult<Vec<PredictionResult>>;
}

/// Statistical model types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatisticalModelType {
    /// Simple moving average
    MovingAverage { window_size: usize },
    /// Linear regression
    LinearRegression,
    /// Exponential smoothing
    ExponentialSmoothing { alpha: f64 },
    /// ARIMA-like model (simplified)
    ARIMA { p: usize, d: usize, q: usize },
}

/// Statistical model implementation
pub struct StatisticalModel {
    model_type: StatisticalModelType,
    name: String,
}

impl StatisticalModel {
    pub fn new(model_type: StatisticalModelType, name: String) -> Self {
        Self { model_type, name }
    }

    /// Calculate moving average
    fn moving_average(&self, data: &[f64], window_size: usize, horizon: u64) -> Vec<f64> {
        if data.len() < window_size {
            return vec![];
        }

        // Calculate the last moving average
        let last_window = &data[data.len() - window_size..];
        let last_avg = last_window.iter().sum::<f64>() / window_size as f64;
        
        // Generate predictions for the full horizon
        let mut predictions = Vec::new();
        for _ in 0..horizon {
            predictions.push(last_avg);
        }
        predictions
    }

    /// Calculate linear regression
    fn linear_regression(&self, data: &[f64], horizon: u64) -> Vec<f64> {
        if data.len() < 2 {
            return vec![];
        }

        let n = data.len() as f64;
        let x_sum: f64 = (0..data.len()).map(|i| i as f64).sum();
        let y_sum: f64 = data.iter().sum();
        let xy_sum: f64 = data.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let x2_sum: f64 = (0..data.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n * xy_sum - x_sum * y_sum) / (n * x2_sum - x_sum.powi(2));
        let intercept = (y_sum - slope * x_sum) / n;

        // Generate predictions for the horizon
        let mut predictions = Vec::new();
        for i in 0..horizon {
            let x = (data.len() as u64 + i) as f64;
            let prediction = slope * x + intercept;
            predictions.push(prediction);
        }

        predictions
    }

    /// Calculate exponential smoothing
    fn exponential_smoothing(&self, data: &[f64], alpha: f64, horizon: u64) -> Vec<f64> {
        if data.is_empty() {
            return vec![];
        }

        let mut smoothed = vec![data[0]];
        for &value in &data[1..] {
            let last_smoothed = smoothed.last().unwrap();
            let new_smoothed = alpha * value + (1.0 - alpha) * last_smoothed;
            smoothed.push(new_smoothed);
        }

        // Generate predictions for the horizon
        let mut predictions = Vec::new();
        let last_smoothed = smoothed.last().unwrap();
        for _ in 0..horizon {
            predictions.push(*last_smoothed);
        }

        predictions
    }

    /// Calculate confidence intervals
    fn calculate_confidence_intervals(
        &self,
        predictions: &[f64],
        historical_data: &[f64],
        confidence_level: f64,
    ) -> Vec<(f64, f64)> {
        if historical_data.len() < 2 {
            return predictions.iter().map(|_| (0.0, 0.0)).collect();
        }

        // Calculate standard deviation of historical data
        let mean = historical_data.iter().sum::<f64>() / historical_data.len() as f64;
        let variance = historical_data
            .iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>()
            / (historical_data.len() - 1) as f64;
        let std_dev = variance.sqrt();

        // Z-score for confidence level (simplified - using 1.96 for 95% confidence)
        let z_score = match confidence_level {
            x if x >= 0.95 => 1.96,
            x if x >= 0.90 => 1.645,
            x if x >= 0.80 => 1.28,
            _ => 1.0,
        };

        let margin_of_error = z_score * std_dev;

        predictions
            .iter()
            .map(|&pred| (pred - margin_of_error, pred + margin_of_error))
            .collect()
    }

    /// Make predictions using the statistical model
    pub fn predict(&self, data: &[f64], horizon: u64) -> BridgeResult<Vec<f64>> {
        let predictions = match &self.model_type {
            StatisticalModelType::MovingAverage { window_size } => {
                self.moving_average(data, *window_size, horizon)
            }
            StatisticalModelType::LinearRegression => self.linear_regression(data, horizon),
            StatisticalModelType::ExponentialSmoothing { alpha } => {
                self.exponential_smoothing(data, *alpha, horizon)
            }
            StatisticalModelType::ARIMA { p, d, q } => {
                // Simplified ARIMA implementation
                self.simplified_arima(data, *p, *d, *q, horizon)
            }
        };

        Ok(predictions)
    }

    /// Simplified ARIMA implementation
    fn simplified_arima(
        &self,
        data: &[f64],
        p: usize,
        d: usize,
        q: usize,
        horizon: u64,
    ) -> Vec<f64> {
        if data.len() < p + d + q {
            return vec![];
        }

        // Simple differencing for d > 0
        let mut differenced = data.to_vec();
        for _ in 0..d {
            let mut new_diff = Vec::new();
            for i in 1..differenced.len() {
                new_diff.push(differenced[i] - differenced[i - 1]);
            }
            differenced = new_diff;
        }

        // Use exponential smoothing as a simple ARIMA approximation
        let alpha = 0.3; // Default smoothing parameter
        self.exponential_smoothing(&differenced, alpha, horizon)
    }
}

/// Default ML predictor implementation
pub struct DefaultMLPredictor {
    config: MLPredictorConfig,
    models: Vec<StatisticalModel>,
}

impl DefaultMLPredictor {
    /// Create a new ML predictor
    pub fn new(config: MLPredictorConfig) -> Self {
        let models = Self::initialize_models(&config);
        Self { config, models }
    }

    /// Initialize statistical models based on configuration
    fn initialize_models(config: &MLPredictorConfig) -> Vec<StatisticalModel> {
        let mut models = Vec::new();

        // Add default models if none are configured
        if config.models.is_empty() {
            models.push(StatisticalModel::new(
                StatisticalModelType::MovingAverage { window_size: 10 },
                "moving_average_10".to_string(),
            ));
            models.push(StatisticalModel::new(
                StatisticalModelType::LinearRegression,
                "linear_regression".to_string(),
            ));
            models.push(StatisticalModel::new(
                StatisticalModelType::ExponentialSmoothing { alpha: 0.3 },
                "exponential_smoothing".to_string(),
            ));
        } else {
            // Initialize models from configuration
            for model_config in &config.models {
                let model_type = match model_config.model_type.as_str() {
                    "moving_average" => {
                        let window_size = model_config
                            .additional_config
                            .get("window_size")
                            .and_then(|s| s.parse::<usize>().ok())
                            .unwrap_or(10);
                        StatisticalModelType::MovingAverage { window_size }
                    }
                    "linear_regression" => StatisticalModelType::LinearRegression,
                    "exponential_smoothing" => {
                        let alpha = model_config
                            .additional_config
                            .get("alpha")
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.3);
                        StatisticalModelType::ExponentialSmoothing { alpha }
                    }
                    "arima" => {
                        let p = model_config
                            .additional_config
                            .get("p")
                            .and_then(|s| s.parse::<usize>().ok())
                            .unwrap_or(1);
                        let d = model_config
                            .additional_config
                            .get("d")
                            .and_then(|s| s.parse::<usize>().ok())
                            .unwrap_or(1);
                        let q = model_config
                            .additional_config
                            .get("q")
                            .and_then(|s| s.parse::<usize>().ok())
                            .unwrap_or(1);
                        StatisticalModelType::ARIMA { p, d, q }
                    }
                    _ => {
                        warn!("Unknown model type: {}, using linear regression", model_config.model_type);
                        StatisticalModelType::LinearRegression
                    }
                };
                models.push(StatisticalModel::new(model_type, model_config.name.clone()));
            }
        }

        models
    }

    /// Extract time series values from data
    fn extract_values(&self, data: &TimeSeriesData) -> Vec<f64> {
        data.points
            .iter()
            .map(|point| point.value)
            .collect()
    }

    /// Generate timestamps for predictions
    fn generate_prediction_timestamps(&self, data: &TimeSeriesData, horizon: u64) -> Vec<DateTime<Utc>> {
        let last_timestamp = data.points.last().map(|p| p.timestamp).unwrap_or_else(Utc::now);
        let mut timestamps = Vec::new();
        
        for i in 1..=horizon {
            let timestamp = last_timestamp + Duration::seconds(i as i64);
            timestamps.push(timestamp);
        }
        
        timestamps
    }

    /// Calculate ensemble prediction
    fn ensemble_prediction(&self, predictions: &[Vec<f64>]) -> Vec<f64> {
        if predictions.is_empty() || predictions[0].is_empty() {
            return vec![];
        }

        let horizon = predictions[0].len();
        let mut ensemble = Vec::new();

        for i in 0..horizon {
            let mut sum = 0.0;
            let mut count = 0;
            
            for model_predictions in predictions {
                if i < model_predictions.len() {
                    sum += model_predictions[i];
                    count += 1;
                }
            }
            
            if count > 0 {
                ensemble.push(sum / count as f64);
            } else {
                ensemble.push(0.0);
            }
        }

        ensemble
    }
}

#[async_trait]
impl MLPredictor for DefaultMLPredictor {
    async fn predict(
        &self,
        data: &TimeSeriesData,
        horizon: u64,
    ) -> BridgeResult<Vec<PredictionResult>> {
        if !self.config.enable_prediction {
            info!("ML prediction is disabled in configuration");
            return Ok(vec![]);
        }

        if data.points.is_empty() {
            warn!("No data points provided for prediction");
            return Ok(vec![]);
        }

        if horizon == 0 {
            warn!("Prediction horizon is 0, no predictions to make");
            return Ok(vec![]);
        }

        info!(
            "Making predictions for time series '{}' with horizon {}",
            data.name, horizon
        );

        let values = self.extract_values(data);
        let prediction_timestamps = self.generate_prediction_timestamps(data, horizon);
        let mut all_predictions = Vec::new();

        // Generate predictions from all models
        for model in &self.models {
            match model.predict(&values, horizon) {
                Ok(predictions) => {
                    debug!(
                        "Model '{}' generated {} predictions",
                        model.name,
                        predictions.len()
                    );
                    all_predictions.push(predictions);
                }
                Err(e) => {
                    error!("Failed to generate predictions with model '{}': {}", model.name, e);
                    continue;
                }
            }
        }

        if all_predictions.is_empty() {
            error!("No models successfully generated predictions");
            return Ok(vec![]);
        }

        // Calculate ensemble prediction
        let ensemble_predictions = self.ensemble_prediction(&all_predictions);
        
        // Calculate confidence intervals for the ensemble
        let confidence_intervals = StatisticalModel::new(
            StatisticalModelType::LinearRegression,
            "ensemble".to_string(),
        )
        .calculate_confidence_intervals(&ensemble_predictions, &values, 0.95);

        // Create prediction results
        let mut results = Vec::new();
        for (i, (prediction, (lower, upper))) in ensemble_predictions
            .iter()
            .zip(confidence_intervals.iter())
            .enumerate()
        {
            let timestamp = prediction_timestamps.get(i).copied().unwrap_or_else(Utc::now);
            
            let result = PredictionResult {
                id: Uuid::new_v4(),
                predicted_value: *prediction,
                confidence_lower: *lower,
                confidence_upper: *upper,
                confidence: 0.95, // Default confidence level
                timestamp,
                model_name: "ensemble".to_string(),
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("horizon_step".to_string(), i.to_string());
                    meta.insert("total_horizon".to_string(), horizon.to_string());
                    meta.insert("models_used".to_string(), self.models.len().to_string());
                    meta
                },
            };
            
            results.push(result);
        }

        info!(
            "Successfully generated {} predictions for time series '{}'",
            results.len(),
            data.name
        );

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::TimeSeriesPoint;
    use chrono::Utc;

    fn create_test_data() -> TimeSeriesData {
        let mut points = Vec::new();
        let base_time = Utc::now();
        
        for i in 0..20 {
            let timestamp = base_time + Duration::seconds(i as i64);
            let value = 10.0 + (i as f64 * 0.5) + (i as f64 * 0.1).sin(); // Linear trend with noise
            points.push(TimeSeriesPoint {
                timestamp,
                value,
                metadata: HashMap::new(),
            });
        }

        TimeSeriesData {
            id: Uuid::new_v4(),
            name: "test_series".to_string(),
            points,
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_default_ml_predictor() {
        let config = MLPredictorConfig {
            name: "test_predictor".to_string(),
            version: "1.0.0".to_string(),
            enable_prediction: true,
            models: vec![],
            additional_config: HashMap::new(),
        };

        let predictor = DefaultMLPredictor::new(config);
        let test_data = create_test_data();
        let horizon = 5;

        let results = predictor.predict(&test_data, horizon).await;
        assert!(results.is_ok());

        let predictions = results.unwrap();
        assert_eq!(predictions.len(), horizon as usize);
        
        for prediction in &predictions {
            assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
            assert!(prediction.confidence_lower <= prediction.confidence_upper);
        }
    }

    #[tokio::test]
    async fn test_statistical_models() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let horizon = 3;

        // Test moving average
        let ma_model = StatisticalModel::new(
            StatisticalModelType::MovingAverage { window_size: 3 },
            "test_ma".to_string(),
        );
        let ma_predictions = ma_model.predict(&data, horizon).unwrap();
        assert!(!ma_predictions.is_empty());

        // Test linear regression
        let lr_model = StatisticalModel::new(
            StatisticalModelType::LinearRegression,
            "test_lr".to_string(),
        );
        let lr_predictions = lr_model.predict(&data, horizon).unwrap();
        assert_eq!(lr_predictions.len(), horizon as usize);

        // Test exponential smoothing
        let es_model = StatisticalModel::new(
            StatisticalModelType::ExponentialSmoothing { alpha: 0.3 },
            "test_es".to_string(),
        );
        let es_predictions = es_model.predict(&data, horizon).unwrap();
        assert_eq!(es_predictions.len(), horizon as usize);
    }
}
