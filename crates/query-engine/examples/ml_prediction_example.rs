//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! ML Prediction Example
//!
//! This example demonstrates how to use the ML prediction functionality
//! in the query engine to make predictions on time series data.

use chrono::{Duration, Utc};
use query_engine::analytics::{
    ml_integration::{DefaultMLPredictor, MLPredictorConfig, MLPredictor},
    TimeSeriesData, TimeSeriesPoint,
};
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== ML Prediction Example ===\n");

    // Create sample time series data with a linear trend and some noise
    let mut points = Vec::new();
    let base_time = Utc::now();
    
    // Generate 30 data points with a linear trend (y = 2x + 10) plus some noise
    for i in 0..30 {
        let timestamp = base_time + Duration::seconds(i as i64);
        let trend = 10.0 + (i as f64 * 2.0);
        let noise = (i as f64 * 0.1).sin() * 0.5; // Small sinusoidal noise
        let value = trend + noise;
        
        points.push(TimeSeriesPoint {
            timestamp,
            value,
            metadata: HashMap::new(),
        });
    }

    let time_series_data = TimeSeriesData {
        id: Uuid::new_v4(),
        name: "sample_metrics".to_string(),
        points,
        metadata: HashMap::new(),
    };

    println!("Generated time series data with {} points", time_series_data.points.len());
    println!("Data range: {:.2} to {:.2}", 
        time_series_data.points.first().unwrap().value,
        time_series_data.points.last().unwrap().value
    );

    // Create ML predictor configuration
    let config = MLPredictorConfig {
        name: "example_predictor".to_string(),
        version: "1.0.0".to_string(),
        enable_prediction: true,
        models: vec![], // Use default models
        additional_config: HashMap::new(),
    };

    // Create the ML predictor
    let predictor = DefaultMLPredictor::new(config);

    // Make predictions for the next 10 time steps
    let horizon = 10;
    println!("\nMaking predictions for the next {} time steps...", horizon);

    let predictions = predictor.predict(&time_series_data, horizon).await?;

    println!("\n=== Prediction Results ===");
    println!("Generated {} predictions:", predictions.len());

    for (i, prediction) in predictions.iter().enumerate() {
        println!(
            "Step {}: {:.2} (confidence: {:.1}%, range: {:.2} - {:.2})",
            i + 1,
            prediction.predicted_value,
            prediction.confidence * 100.0,
            prediction.confidence_lower,
            prediction.confidence_upper
        );
    }

    // Demonstrate different model configurations
    println!("\n=== Custom Model Configuration ===");
    
    let custom_config = MLPredictorConfig {
        name: "custom_predictor".to_string(),
        version: "1.0.0".to_string(),
        enable_prediction: true,
        models: vec![
            query_engine::analytics::ml_integration::MLModelConfig {
                name: "moving_average_5".to_string(),
                version: "1.0.0".to_string(),
                model_type: "moving_average".to_string(),
                model_path: "".to_string(),
                additional_config: {
                    let mut config = HashMap::new();
                    config.insert("window_size".to_string(), "5".to_string());
                    config
                },
            },
            query_engine::analytics::ml_integration::MLModelConfig {
                name: "exponential_smoothing".to_string(),
                version: "1.0.0".to_string(),
                model_type: "exponential_smoothing".to_string(),
                model_path: "".to_string(),
                additional_config: {
                    let mut config = HashMap::new();
                    config.insert("alpha".to_string(), "0.2".to_string());
                    config
                },
            },
        ],
        additional_config: HashMap::new(),
    };

    let custom_predictor = DefaultMLPredictor::new(custom_config);
    let custom_predictions = custom_predictor.predict(&time_series_data, 5).await?;

    println!("Custom model predictions (5 steps):");
    for (i, prediction) in custom_predictions.iter().enumerate() {
        println!(
            "Step {}: {:.2} (confidence: {:.1}%)",
            i + 1,
            prediction.predicted_value,
            prediction.confidence * 100.0
        );
    }

    // Demonstrate error handling
    println!("\n=== Error Handling ===");
    
    // Test with empty data
    let empty_data = TimeSeriesData {
        id: Uuid::new_v4(),
        name: "empty_series".to_string(),
        points: vec![],
        metadata: HashMap::new(),
    };

    let empty_result = predictor.predict(&empty_data, 5).await;
    match empty_result {
        Ok(predictions) => {
            println!("Empty data result: {} predictions", predictions.len());
        }
        Err(e) => {
            println!("Empty data error: {}", e);
        }
    }

    // Test with zero horizon
    let zero_horizon_result = predictor.predict(&time_series_data, 0).await;
    match zero_horizon_result {
        Ok(predictions) => {
            println!("Zero horizon result: {} predictions", predictions.len());
        }
        Err(e) => {
            println!("Zero horizon error: {}", e);
        }
    }

    println!("\n=== Example Complete ===");
    println!("The ML prediction system successfully:");
    println!("1. Generated time series data with trends and noise");
    println!("2. Created predictions using ensemble of statistical models");
    println!("3. Provided confidence intervals for predictions");
    println!("4. Demonstrated custom model configuration");
    println!("5. Handled edge cases gracefully");

    Ok(())
}
