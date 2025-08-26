//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Anomaly Detection Example
//!
//! This example demonstrates how to use the anomaly detection functionality
//! in the query engine to detect various types of anomalies in time series data.

use chrono::{Duration, Utc};
use query_engine::analytics::{
    anomaly_detection::{DefaultAnomalyDetector, AnomalyDetectorConfig, AnomalyDetector},
    TimeSeriesData, TimeSeriesPoint,
};
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Anomaly Detection Example ===\n");

    // Create sample time series data with various types of anomalies
    let mut points = Vec::new();
    let base_time = Utc::now();
    
    // Generate 50 data points with different types of anomalies
    for i in 0..50 {
        let timestamp = base_time + Duration::seconds(i as i64);
        let value = match i {
            // Normal baseline pattern
            0..=9 => 10.0 + (i as f64 * 0.1).sin(),
            
            // Point anomaly - sudden spike
            10 => 50.0, // Obvious spike
            
            // Normal pattern continues
            11..=19 => 10.0 + (i as f64 * 0.1).sin(),
            
            // Contextual anomaly - value that's normal in isolation but anomalous in context
            20..=25 => 15.0 + (i as f64 * 0.2).sin(), // Higher baseline
            
            // Normal pattern
            26..=29 => 10.0 + (i as f64 * 0.1).sin(),
            
            // Trend anomaly - gradual change that becomes anomalous
            30..=35 => 10.0 + (i as f64 * 2.0), // Steep upward trend
            
            // Normal pattern
            36..=39 => 10.0 + (i as f64 * 0.1).sin(),
            
            // Seasonal anomaly - pattern that breaks seasonal expectation
            40..=45 => 5.0 + (i as f64 * 0.1).sin(), // Lower baseline
            
            // Normal pattern
            46..=49 => 10.0 + (i as f64 * 0.1).sin(),
            
            _ => 10.0 + (i as f64 * 0.1).sin(),
        };
        
        points.push(TimeSeriesPoint {
            timestamp,
            value,
            metadata: HashMap::new(),
        });
    }

    let time_series_data = TimeSeriesData {
        id: Uuid::new_v4(),
        name: "sample_metrics_with_anomalies".to_string(),
        points,
        metadata: HashMap::new(),
    };

    println!("Generated time series data with {} points", time_series_data.points.len());
    println!("Data range: {:.2} to {:.2}", 
        time_series_data.points.first().unwrap().value,
        time_series_data.points.last().unwrap().value
    );

    // Create anomaly detector configuration with default methods
    let config = AnomalyDetectorConfig {
        name: "example_detector".to_string(),
        version: "1.0.0".to_string(),
        enable_detection: true,
        threshold: 2.0,
        window_size: 10,
        additional_config: HashMap::new(),
    };

    // Create the anomaly detector
    let detector = DefaultAnomalyDetector::new(config);

    // Detect anomalies
    println!("\nDetecting anomalies...");

    let anomalies = detector.detect_anomalies(&time_series_data).await?;

    println!("\n=== Anomaly Detection Results ===");
    println!("Found {} anomalies:", anomalies.len());

    for (i, anomaly) in anomalies.iter().enumerate() {
        println!(
            "\nAnomaly {}:",
            i + 1
        );
        println!("  Type: {:?}", anomaly.anomaly_type);
        println!("  Timestamp: {}", anomaly.timestamp);
        println!("  Value: {:.2}", anomaly.metadata.get("value").unwrap_or(&"N/A".to_string()));
        println!("  Confidence: {:.1}%", anomaly.confidence * 100.0);
        println!("  Severity: {:.1}%", anomaly.severity * 100.0);
        println!("  Description: {}", anomaly.description);
        println!("  Detector: {}", anomaly.metadata.get("detector").unwrap_or(&"Unknown".to_string()));
    }

    // Demonstrate custom detection methods
    println!("\n=== Custom Detection Methods ===");
    
    let custom_config = AnomalyDetectorConfig {
        name: "custom_detector".to_string(),
        version: "1.0.0".to_string(),
        enable_detection: true,
        threshold: 1.5,
        window_size: 5,
        additional_config: {
            let mut config = HashMap::new();
            config.insert("z_score".to_string(), "2.5".to_string());
            config.insert("iqr".to_string(), "2.0".to_string());
            config.insert("trend".to_string(), "8,0.3".to_string()); // window_size,threshold
            config.insert("seasonal".to_string(), "10,0.5".to_string()); // period,threshold
            config
        },
    };

    let custom_detector = DefaultAnomalyDetector::new(custom_config);
    let custom_anomalies = custom_detector.detect_anomalies(&time_series_data).await?;

    println!("Custom detector found {} anomalies:", custom_anomalies.len());
    
    // Group anomalies by type
    let mut anomalies_by_type: HashMap<String, Vec<&query_engine::analytics::AnomalyResult>> = HashMap::new();
    for anomaly in &custom_anomalies {
        let detector_name = anomaly.metadata.get("detector").unwrap_or(&"Unknown".to_string()).clone();
        anomalies_by_type.entry(detector_name).or_insert_with(Vec::new).push(anomaly);
    }

    for (detector_name, detector_anomalies) in anomalies_by_type {
        println!("  {}: {} anomalies", detector_name, detector_anomalies.len());
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

    let empty_result = detector.detect_anomalies(&empty_data).await;
    match empty_result {
        Ok(anomalies) => {
            println!("Empty data result: {} anomalies", anomalies.len());
        }
        Err(e) => {
            println!("Empty data error: {}", e);
        }
    }

    // Test with disabled detection
    let disabled_config = AnomalyDetectorConfig {
        name: "disabled_detector".to_string(),
        version: "1.0.0".to_string(),
        enable_detection: false,
        threshold: 2.0,
        window_size: 10,
        additional_config: HashMap::new(),
    };

    let disabled_detector = DefaultAnomalyDetector::new(disabled_config);
    let disabled_result = disabled_detector.detect_anomalies(&time_series_data).await?;
    println!("Disabled detector result: {} anomalies", disabled_result.len());

    // Show anomaly statistics
    println!("\n=== Anomaly Statistics ===");
    if !anomalies.is_empty() {
        let avg_confidence: f64 = anomalies.iter().map(|a| a.confidence).sum::<f64>() / anomalies.len() as f64;
        let avg_severity: f64 = anomalies.iter().map(|a| a.severity).sum::<f64>() / anomalies.len() as f64;
        let max_severity = anomalies.iter().map(|a| a.severity).fold(0.0, f64::max);
        
        println!("Average confidence: {:.1}%", avg_confidence * 100.0);
        println!("Average severity: {:.1}%", avg_severity * 100.0);
        println!("Maximum severity: {:.1}%", max_severity * 100.0);
        
        // Count by type
        let mut type_counts: HashMap<String, usize> = HashMap::new();
        for anomaly in &anomalies {
            let type_name = format!("{:?}", anomaly.anomaly_type);
            *type_counts.entry(type_name).or_insert(0) += 1;
        }
        
        println!("Anomalies by type:");
        for (anomaly_type, count) in type_counts {
            println!("  {}: {}", anomaly_type, count);
        }
    }

    println!("\n=== Example Complete ===");
    println!("The anomaly detection system successfully:");
    println!("1. Generated time series data with various anomaly types");
    println!("2. Detected anomalies using multiple statistical methods");
    println!("3. Provided confidence and severity scores");
    println!("4. Demonstrated custom detection method configuration");
    println!("5. Handled edge cases gracefully");
    println!("6. Analyzed anomaly statistics and distributions");

    Ok(())
}
