//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Anomaly detection for time series data
//!
//! This module provides anomaly detection capabilities for time series data.

use async_trait::async_trait;
use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{AnomalyResult, AnomalyType, TimeSeriesData, TimeSeriesPoint};

/// Anomaly detector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetectorConfig {
    /// Detector name
    pub name: String,

    /// Detector version
    pub version: String,

    /// Enable detection
    pub enable_detection: bool,

    /// Detection threshold
    pub threshold: f64,

    /// Window size for detection
    pub window_size: usize,

    /// Additional configuration
    pub additional_config: HashMap<String, String>,
}

impl Default for AnomalyDetectorConfig {
    fn default() -> Self {
        Self {
            name: "default_anomaly_detector".to_string(),
            version: "1.0.0".to_string(),
            enable_detection: true,
            threshold: 3.0,
            window_size: 10,
            additional_config: HashMap::new(),
        }
    }
}

/// Anomaly detector trait
#[async_trait]
pub trait AnomalyDetector: Send + Sync {
    /// Detect anomalies in time series data
    async fn detect_anomalies(&self, data: &TimeSeriesData) -> BridgeResult<Vec<AnomalyResult>>;
}

/// Detection method types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionMethod {
    /// Z-score based detection
    ZScore { threshold: f64 },
    /// Moving average based detection
    MovingAverage { window_size: usize, threshold: f64 },
    /// Interquartile range (IQR) based detection
    IQR { multiplier: f64 },
    /// Trend-based detection
    Trend { window_size: usize, threshold: f64 },
    /// Seasonal decomposition based detection
    Seasonal { period: usize, threshold: f64 },
}

/// Statistical anomaly detector implementation
pub struct StatisticalAnomalyDetector {
    method: DetectionMethod,
    name: String,
}

impl StatisticalAnomalyDetector {
    pub fn new(method: DetectionMethod, name: String) -> Self {
        Self { method, name }
    }

    /// Z-score based anomaly detection
    fn z_score_detection(&self, data: &[f64], threshold: f64) -> Vec<(usize, f64, f64)> {
        if data.len() < 2 {
            return vec![];
        }

        let mean = data.iter().sum::<f64>() / data.len() as f64;
        let variance = data.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (data.len() - 1) as f64;
        let std_dev = variance.sqrt();

        if std_dev == 0.0 {
            return vec![];
        }

        let mut anomalies = Vec::new();
        for (i, &value) in data.iter().enumerate() {
            let z_score = (value - mean).abs() / std_dev;
            if z_score > threshold {
                anomalies.push((i, value, z_score));
            }
        }

        anomalies
    }

    /// Moving average based anomaly detection
    fn moving_average_detection(&self, data: &[f64], window_size: usize, threshold: f64) -> Vec<(usize, f64, f64)> {
        if data.len() < window_size + 1 {
            return vec![];
        }

        let mut anomalies = Vec::new();
        for i in window_size..data.len() {
            let window = &data[i - window_size..i];
            let moving_avg = window.iter().sum::<f64>() / window_size as f64;
            let deviation = (data[i] - moving_avg).abs() / moving_avg;
            
            if deviation > threshold {
                anomalies.push((i, data[i], deviation));
            }
        }

        anomalies
    }

    /// IQR based anomaly detection
    fn iqr_detection(&self, data: &[f64], multiplier: f64) -> Vec<(usize, f64, f64)> {
        if data.len() < 4 {
            return vec![];
        }

        let mut sorted_data = data.to_vec();
        sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let q1_idx = (sorted_data.len() as f64 * 0.25) as usize;
        let q3_idx = (sorted_data.len() as f64 * 0.75) as usize;
        
        let q1 = sorted_data[q1_idx];
        let q3 = sorted_data[q3_idx];
        let iqr = q3 - q1;
        
        let lower_bound = q1 - multiplier * iqr;
        let upper_bound = q3 + multiplier * iqr;

        let mut anomalies = Vec::new();
        for (i, &value) in data.iter().enumerate() {
            if value < lower_bound || value > upper_bound {
                let deviation = if value < lower_bound {
                    (lower_bound - value) / iqr
                } else {
                    (value - upper_bound) / iqr
                };
                anomalies.push((i, value, deviation));
            }
        }

        anomalies
    }

    /// Trend-based anomaly detection
    fn trend_detection(&self, data: &[f64], window_size: usize, threshold: f64) -> Vec<(usize, f64, f64)> {
        if data.len() < window_size * 2 {
            return vec![];
        }

        let mut anomalies = Vec::new();
        for i in window_size..data.len() {
            let window = &data[i - window_size..i];
            let trend = self.calculate_trend(window);
            let current_value = data[i];
            let expected_value = window.last().unwrap() + trend;
            let deviation = (current_value - expected_value).abs() / expected_value.abs().max(1e-10);
            
            if deviation > threshold {
                anomalies.push((i, current_value, deviation));
            }
        }

        anomalies
    }

    /// Calculate trend in a window of data
    fn calculate_trend(&self, window: &[f64]) -> f64 {
        if window.len() < 2 {
            return 0.0;
        }

        let n = window.len() as f64;
        let x_sum: f64 = (0..window.len()).map(|i| i as f64).sum();
        let y_sum: f64 = window.iter().sum();
        let xy_sum: f64 = window.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let x2_sum: f64 = (0..window.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n * xy_sum - x_sum * y_sum) / (n * x2_sum - x_sum.powi(2));
        slope
    }

    /// Seasonal decomposition based detection (simplified)
    fn seasonal_detection(&self, data: &[f64], period: usize, threshold: f64) -> Vec<(usize, f64, f64)> {
        if data.len() < period * 2 {
            return vec![];
        }

        let mut anomalies = Vec::new();
        for i in period..data.len() {
            let seasonal_value = data[i - period];
            let current_value = data[i];
            let deviation = (current_value - seasonal_value).abs() / seasonal_value.abs().max(1e-10);
            
            if deviation > threshold {
                anomalies.push((i, current_value, deviation));
            }
        }

        anomalies
    }

    /// Detect anomalies using the configured method
    pub fn detect(&self, data: &[f64]) -> Vec<(usize, f64, f64)> {
        match &self.method {
            DetectionMethod::ZScore { threshold } => {
                self.z_score_detection(data, *threshold)
            }
            DetectionMethod::MovingAverage { window_size, threshold } => {
                self.moving_average_detection(data, *window_size, *threshold)
            }
            DetectionMethod::IQR { multiplier } => {
                self.iqr_detection(data, *multiplier)
            }
            DetectionMethod::Trend { window_size, threshold } => {
                self.trend_detection(data, *window_size, *threshold)
            }
            DetectionMethod::Seasonal { period, threshold } => {
                self.seasonal_detection(data, *period, *threshold)
            }
        }
    }
}

/// Default anomaly detector implementation
pub struct DefaultAnomalyDetector {
    config: AnomalyDetectorConfig,
    detectors: Vec<StatisticalAnomalyDetector>,
}

impl DefaultAnomalyDetector {
    /// Create a new anomaly detector
    pub fn new(config: AnomalyDetectorConfig) -> Self {
        let detectors = Self::initialize_detectors(&config);
        Self { config, detectors }
    }

    /// Initialize detection methods based on configuration
    fn initialize_detectors(config: &AnomalyDetectorConfig) -> Vec<StatisticalAnomalyDetector> {
        let mut detectors = Vec::new();

        // Add default detectors if none are configured
        if config.additional_config.is_empty() {
            detectors.push(StatisticalAnomalyDetector::new(
                DetectionMethod::ZScore { threshold: 3.0 },
                "z_score_detector".to_string(),
            ));
            detectors.push(StatisticalAnomalyDetector::new(
                DetectionMethod::MovingAverage { 
                    window_size: config.window_size, 
                    threshold: config.threshold 
                },
                "moving_average_detector".to_string(),
            ));
            detectors.push(StatisticalAnomalyDetector::new(
                DetectionMethod::IQR { multiplier: 1.5 },
                "iqr_detector".to_string(),
            ));
        } else {
            // Initialize detectors from configuration
            for (method_name, params) in &config.additional_config {
                let detector = match method_name.as_str() {
                    "z_score" => {
                        let threshold = params.parse::<f64>().unwrap_or(3.0);
                        StatisticalAnomalyDetector::new(
                            DetectionMethod::ZScore { threshold },
                            "z_score_detector".to_string(),
                        )
                    }
                    "moving_average" => {
                        let parts: Vec<&str> = params.split(',').collect();
                        let window_size = parts.get(0).and_then(|s| s.parse::<usize>().ok()).unwrap_or(config.window_size);
                        let threshold = parts.get(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(config.threshold);
                        StatisticalAnomalyDetector::new(
                            DetectionMethod::MovingAverage { window_size, threshold },
                            "moving_average_detector".to_string(),
                        )
                    }
                    "iqr" => {
                        let multiplier = params.parse::<f64>().unwrap_or(1.5);
                        StatisticalAnomalyDetector::new(
                            DetectionMethod::IQR { multiplier },
                            "iqr_detector".to_string(),
                        )
                    }
                    "trend" => {
                        let parts: Vec<&str> = params.split(',').collect();
                        let window_size = parts.get(0).and_then(|s| s.parse::<usize>().ok()).unwrap_or(config.window_size);
                        let threshold = parts.get(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(config.threshold);
                        StatisticalAnomalyDetector::new(
                            DetectionMethod::Trend { window_size, threshold },
                            "trend_detector".to_string(),
                        )
                    }
                    "seasonal" => {
                        let parts: Vec<&str> = params.split(',').collect();
                        let period = parts.get(0).and_then(|s| s.parse::<usize>().ok()).unwrap_or(24);
                        let threshold = parts.get(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(config.threshold);
                        StatisticalAnomalyDetector::new(
                            DetectionMethod::Seasonal { period, threshold },
                            "seasonal_detector".to_string(),
                        )
                    }
                    _ => {
                        warn!("Unknown detection method: {}, using z-score", method_name);
                        StatisticalAnomalyDetector::new(
                            DetectionMethod::ZScore { threshold: 3.0 },
                            "z_score_detector".to_string(),
                        )
                    }
                };
                detectors.push(detector);
            }
        }

        detectors
    }

    /// Extract time series values from data
    fn extract_values(&self, data: &TimeSeriesData) -> Vec<f64> {
        data.points
            .iter()
            .map(|point| point.value)
            .collect()
    }

    /// Determine anomaly type based on detection method and context
    fn determine_anomaly_type(&self, detector_name: &str, index: usize, data: &[f64]) -> AnomalyType {
        match detector_name {
            "z_score_detector" | "iqr_detector" => AnomalyType::Point,
            "moving_average_detector" => AnomalyType::Contextual,
            "trend_detector" => AnomalyType::Trend,
            "seasonal_detector" => AnomalyType::Seasonal,
            _ => {
                // Analyze context to determine type
                if index > 0 && index < data.len() - 1 {
                    let prev = data[index - 1];
                    let curr = data[index];
                    let next = data[index + 1];
                    
                    if (curr - prev).abs() > (next - curr).abs() * 2.0 {
                        AnomalyType::Point
                    } else {
                        AnomalyType::Contextual
                    }
                } else {
                    AnomalyType::Point
                }
            }
        }
    }

    /// Calculate severity score based on deviation
    fn calculate_severity(&self, deviation: f64, threshold: f64) -> f64 {
        let normalized = (deviation / threshold).min(10.0); // Cap at 10x threshold
        (normalized / 10.0).min(1.0) // Normalize to 0-1 range
    }

    /// Generate anomaly description
    fn generate_description(&self, anomaly_type: &AnomalyType, value: f64, deviation: f64) -> String {
        match anomaly_type {
            AnomalyType::Point => format!("Point anomaly detected: value {:.2} (deviation: {:.2})", value, deviation),
            AnomalyType::Contextual => format!("Contextual anomaly detected: value {:.2} (deviation: {:.2})", value, deviation),
            AnomalyType::Collective => format!("Collective anomaly detected: value {:.2} (deviation: {:.2})", value, deviation),
            AnomalyType::Trend => format!("Trend anomaly detected: value {:.2} (deviation: {:.2})", value, deviation),
            AnomalyType::Seasonal => format!("Seasonal anomaly detected: value {:.2} (deviation: {:.2})", value, deviation),
        }
    }
}

#[async_trait]
impl AnomalyDetector for DefaultAnomalyDetector {
    async fn detect_anomalies(&self, data: &TimeSeriesData) -> BridgeResult<Vec<AnomalyResult>> {
        if !self.config.enable_detection {
            info!("Anomaly detection is disabled in configuration");
            return Ok(vec![]);
        }

        if data.points.is_empty() {
            warn!("No data points provided for anomaly detection");
            return Ok(vec![]);
        }

        info!(
            "Detecting anomalies in time series '{}' with {} points",
            data.name,
            data.points.len()
        );

        let values = self.extract_values(data);
        let mut all_anomalies = Vec::new();

        // Run all detection methods
        for detector in &self.detectors {
            match detector.detect(&values) {
                anomalies => {
                    debug!(
                        "Detector '{}' found {} anomalies",
                        detector.name,
                        anomalies.len()
                    );
                    
                    for (index, value, deviation) in anomalies {
                        let anomaly_type = self.determine_anomaly_type(&detector.name, index, &values);
                        let severity = self.calculate_severity(deviation, self.config.threshold);
                        let confidence = (1.0 - (deviation / (deviation + 1.0))).min(1.0);
                        let description = self.generate_description(&anomaly_type, value, deviation);
                        
                        let timestamp = data.points.get(index).map(|p| p.timestamp).unwrap_or_else(Utc::now);
                        
                        let result = AnomalyResult {
                            id: Uuid::new_v4(),
                            anomaly_type,
                            timestamp,
                            confidence,
                            severity,
                            description,
                            metadata: {
                                let mut meta = HashMap::new();
                                meta.insert("detector".to_string(), detector.name.clone());
                                meta.insert("value".to_string(), value.to_string());
                                meta.insert("deviation".to_string(), deviation.to_string());
                                meta.insert("index".to_string(), index.to_string());
                                meta
                            },
                        };
                        
                        all_anomalies.push(result);
                    }
                }
            }
        }

        // Sort anomalies by timestamp
        all_anomalies.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        info!(
            "Successfully detected {} anomalies in time series '{}'",
            all_anomalies.len(),
            data.name
        );

        Ok(all_anomalies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_data() -> TimeSeriesData {
        let mut points = Vec::new();
        let base_time = Utc::now();
        
        // Create data with some obvious anomalies
        for i in 0..20 {
            let timestamp = base_time + chrono::Duration::seconds(i as i64);
            let value = if i == 10 {
                100.0 // Obvious anomaly
            } else {
                10.0 + (i as f64 * 0.1).sin() // Normal pattern
            };
            
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
    async fn test_default_anomaly_detector() {
        let config = AnomalyDetectorConfig {
            name: "test_detector".to_string(),
            version: "1.0.0".to_string(),
            enable_detection: true,
            threshold: 2.0,
            window_size: 5,
            additional_config: HashMap::new(),
        };

        let detector = DefaultAnomalyDetector::new(config);
        let test_data = create_test_data();

        let results = detector.detect_anomalies(&test_data).await;
        assert!(results.is_ok());

        let anomalies = results.unwrap();
        assert!(!anomalies.is_empty());
        
        for anomaly in &anomalies {
            assert!(anomaly.confidence >= 0.0 && anomaly.confidence <= 1.0);
            assert!(anomaly.severity >= 0.0 && anomaly.severity <= 1.0);
            assert!(!anomaly.description.is_empty());
        }
    }

    #[tokio::test]
    async fn test_statistical_detectors() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 100.0, 6.0, 7.0, 8.0, 9.0, 10.0]; // 100 is anomaly

        // Test Z-score detector
        let z_detector = StatisticalAnomalyDetector::new(
            DetectionMethod::ZScore { threshold: 2.0 },
            "test_z".to_string(),
        );
        let z_anomalies = z_detector.detect(&data);
        assert!(!z_anomalies.is_empty());

        // Test IQR detector
        let iqr_detector = StatisticalAnomalyDetector::new(
            DetectionMethod::IQR { multiplier: 1.5 },
            "test_iqr".to_string(),
        );
        let iqr_anomalies = iqr_detector.detect(&data);
        assert!(!iqr_anomalies.is_empty());

        // Test moving average detector
        let ma_detector = StatisticalAnomalyDetector::new(
            DetectionMethod::MovingAverage { window_size: 3, threshold: 0.5 },
            "test_ma".to_string(),
        );
        let ma_anomalies = ma_detector.detect(&data);
        assert!(!ma_anomalies.is_empty());
    }

    #[tokio::test]
    async fn test_empty_data() {
        let config = AnomalyDetectorConfig {
            name: "test_detector".to_string(),
            version: "1.0.0".to_string(),
            enable_detection: true,
            threshold: 2.0,
            window_size: 5,
            additional_config: HashMap::new(),
        };

        let detector = DefaultAnomalyDetector::new(config);
        let empty_data = TimeSeriesData {
            id: Uuid::new_v4(),
            name: "empty_series".to_string(),
            points: vec![],
            metadata: HashMap::new(),
        };

        let results = detector.detect_anomalies(&empty_data).await;
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 0);
    }
}
