//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Example demonstrating streaming queries, query plan visualization, and advanced analytics
//!
//! This example shows how to use the new Phase 3 features:
//! - Streaming Queries: Real-time data processing capabilities
//! - Query Plan Visualization: Tools for analyzing and optimizing query performance
//! - Advanced Analytics: Time series analysis and machine learning integration

use bridge_core::BridgeResult;
use chrono::{DateTime, Utc};
use query_engine::{
    analytics::{
        anomaly_detection::{AnomalyDetector, DefaultAnomalyDetector},
        clustering::DefaultClusterAnalyzer,
        forecasting::DefaultForecaster,
        ml_integration::{DefaultMLPredictor, MLPredictor},
        time_series::TimeSeriesAnalyzer,
    },
    executors::MockExecutor,
    streaming::{
        ContinuousQuery, ContinuousQueryConfig, ContinuousQueryManager, StreamingQueryConfig,
        StreamingQueryManager, WindowConfig, WindowType,
    },
    AnalyticsConfig, AnomalyDetectorConfig, MLPredictorConfig, PlanVisualizer,
    PlanVisualizerConfig, QueryEngineConfig, QueryPlanEdge, QueryPlanGraph, QueryPlanNode,
    TimeSeriesAnalyzerConfig, TimeSeriesData, TimeSeriesPoint, VisualizationFormat, Visualizer,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{error, info};
use uuid::Uuid;

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting Phase 3 Features Demo");

    // Demo 1: Streaming Queries
    demo_streaming_queries().await?;

    // Demo 2: Query Plan Visualization
    demo_query_plan_visualization().await?;

    // Demo 3: Advanced Analytics
    demo_advanced_analytics().await?;

    info!("Phase 3 Features Demo completed successfully");
    Ok(())
}

/// Demo 1: Streaming Queries
async fn demo_streaming_queries() -> BridgeResult<()> {
    info!("=== Demo 1: Streaming Queries ===");

    // Create streaming query configuration
    let streaming_config = StreamingQueryConfig::default();

    // Create continuous query configuration
    let continuous_config = ContinuousQueryConfig {
        name: "service_latency_monitor".to_string(),
        sql: "SELECT service_name, AVG(response_time) as avg_latency FROM telemetry WHERE timestamp >= NOW() - INTERVAL '1 hour' GROUP BY service_name".to_string(),
        execution_interval_ms: 5000, // Execute every 5 seconds
                    window_config: Some(query_engine::streaming::continuous_queries::WindowConfig {
                window_type: query_engine::streaming::windowing::WindowType::Time,
                window_size_ms: 60000, // 1 minute window
                window_slide_ms: 30000, // 30 second slide
                enable_watermarking: true,
                watermark_delay_ms: 5000,
            }),
        enable_result_streaming: true,
        result_buffer_size: 1000,
        max_execution_time_secs: 30,
        enable_optimization: true,
        additional_config: HashMap::new(),
    };

    // Create query executor
    let executor = Arc::new(MockExecutor::new());

    // Create continuous query
    let mut continuous_query = ContinuousQuery::new(continuous_config, streaming_config, executor);

    // Start the continuous query
    continuous_query.start().await?;
    info!("Started continuous query: service_latency_monitor");

    // Create continuous query manager
    let mut query_manager = ContinuousQueryManager::new();

    // Register the query
    query_manager
        .register_query(Box::new(continuous_query))
        .await?;
    info!("Registered continuous query with manager");

    // Get active queries
    let active_queries = query_manager.get_active_queries().await?;
    info!("Active queries: {}", active_queries.len());

    // Get manager statistics
    let stats = query_manager.get_stats().await?;
    info!("Query manager stats: {:?}", stats);

    // Simulate some time for the query to run
    sleep(Duration::from_secs(10)).await;

    // Stop all queries
    query_manager.stop_all().await?;
    info!("Stopped all continuous queries");

    Ok(())
}

/// Demo 2: Query Plan Visualization
async fn demo_query_plan_visualization() -> BridgeResult<()> {
    info!("=== Demo 2: Query Plan Visualization ===");

    // Create a sample query plan graph
    let query_plan = create_sample_query_plan().await?;

    // Create plan visualizer configuration
    let visualizer_config = PlanVisualizerConfig {
        name: "demo_visualizer".to_string(),
        version: "1.0.0".to_string(),
        enable_visualization: true,
        default_format: VisualizationFormat::Dot,
        output_directory: "./visualizations".to_string(),
        enable_color_coding: true,
        enable_cost_annotations: true,
        enable_performance_annotations: true,
        additional_config: HashMap::new(),
    };

    // Create plan visualizer
    let visualizer = PlanVisualizer::new(visualizer_config);

    // Generate visualizations in different formats
    let formats = vec![
        VisualizationFormat::Dot,
        VisualizationFormat::Json,
        VisualizationFormat::Html,
    ];

    for format in formats {
        let filename = format!("query_plan_demo.{}", get_format_extension(&format));
        let path = format!("./visualizations/{}", filename);

        // Save visualization to file
        visualizer
            .save_visualization(&query_plan, &format, &path)
            .await?;
        info!("Saved {} visualization to: {}", format_name(&format), path);

        // Generate visualization content
        let content = visualizer.visualize(&query_plan, &format).await?;
        info!(
            "Generated {} visualization ({} bytes)",
            format_name(&format),
            content.len()
        );
    }

    // Get supported formats
    let supported_formats = visualizer.get_supported_formats();
    info!("Supported visualization formats: {:?}", supported_formats);

    Ok(())
}

/// Demo 3: Advanced Analytics
async fn demo_advanced_analytics() -> BridgeResult<()> {
    info!("=== Demo 3: Advanced Analytics ===");

    // Create analytics configuration
    let analytics_config = AnalyticsConfig::default();

    // Create sample time series data
    let time_series_data = create_sample_time_series_data().await?;

    // Demo 3a: Time Series Analysis
    demo_time_series_analysis(&time_series_data).await?;

    // Demo 3b: Anomaly Detection
    demo_anomaly_detection(&time_series_data).await?;

    // Demo 3c: Machine Learning Integration
    demo_ml_integration(&time_series_data).await?;

    Ok(())
}

/// Demo 3a: Time Series Analysis
async fn demo_time_series_analysis(data: &TimeSeriesData) -> BridgeResult<()> {
    info!("--- Time Series Analysis ---");

    // Create time series analyzer configuration
    let analyzer_config = TimeSeriesAnalyzerConfig::default();
    let analyzer = TimeSeriesAnalyzer::new(analyzer_config.clone());

    // For now, just print the analyzer configuration
    info!(
        "Time series analyzer created with config: {:?}",
        analyzer_config
    );

    info!("Time series analysis completed:");
    info!("  - Analyzer created successfully");
    info!("  - Configuration: {:?}", analyzer_config);

    Ok(())
}

/// Demo 3b: Anomaly Detection
async fn demo_anomaly_detection(data: &TimeSeriesData) -> BridgeResult<()> {
    info!("--- Anomaly Detection ---");

    // Create anomaly detector configuration
    let detector_config = AnomalyDetectorConfig {
        name: "demo_detector".to_string(),
        version: "1.0.0".to_string(),
        enable_detection: true,
        threshold: 2.0,
        window_size: 100,
        additional_config: HashMap::new(),
    };

    // Create anomaly detector
    let detector = DefaultAnomalyDetector::new(detector_config);

    // Detect anomalies
    let anomalies = detector.detect_anomalies(data).await?;

    info!("Anomaly detection completed:");
    info!("  - Anomalies detected: {}", anomalies.len());

    for (i, anomaly) in anomalies.iter().enumerate() {
        info!(
            "  - Anomaly {}: {:?} at {} (confidence: {:.2})",
            i + 1,
            anomaly.anomaly_type,
            anomaly.timestamp,
            anomaly.confidence
        );
    }

    Ok(())
}

/// Demo 3c: Machine Learning Integration
async fn demo_ml_integration(data: &TimeSeriesData) -> BridgeResult<()> {
    info!("--- Machine Learning Integration ---");

    // Create ML predictor configuration
    let predictor_config = MLPredictorConfig {
        name: "demo_predictor".to_string(),
        version: "1.0.0".to_string(),
        enable_prediction: true,
        models: vec![],
        additional_config: HashMap::new(),
    };

    // Create ML predictor
    let predictor = DefaultMLPredictor::new(predictor_config);

    // Make predictions
    let predictions = predictor.predict(data, 10).await?;

    info!("ML prediction completed:");
    info!("  - Predictions generated: {}", predictions.len());

    for (i, prediction) in predictions.iter().enumerate() {
        info!(
            "  - Prediction {}: {:.2} (confidence: {:.2})",
            i + 1,
            prediction.predicted_value,
            prediction.confidence
        );
    }

    Ok(())
}

/// Create a sample query plan graph
async fn create_sample_query_plan() -> BridgeResult<QueryPlanGraph> {
    let scan_node = QueryPlanNode {
        id: Uuid::new_v4(),
        node_type: "TableScan".to_string(),
        name: "telemetry_scan".to_string(),
        description: "Scan telemetry table".to_string(),
        estimated_cost: 100.0,
        actual_cost: Some(95.0),
        estimated_rows: 10000,
        actual_rows: Some(9850),
        execution_time_ms: Some(50),
        metadata: HashMap::new(),
        children: vec![],
    };

    let filter_node = QueryPlanNode {
        id: Uuid::new_v4(),
        node_type: "Filter".to_string(),
        name: "timestamp_filter".to_string(),
        description: "Filter by timestamp".to_string(),
        estimated_cost: 50.0,
        actual_cost: Some(45.0),
        estimated_rows: 5000,
        actual_rows: Some(4800),
        execution_time_ms: Some(25),
        metadata: HashMap::new(),
        children: vec![scan_node.id],
    };

    let aggregate_node = QueryPlanNode {
        id: Uuid::new_v4(),
        node_type: "Aggregate".to_string(),
        name: "service_aggregate".to_string(),
        description: "Group by service_name".to_string(),
        estimated_cost: 25.0,
        actual_cost: Some(22.0),
        estimated_rows: 100,
        actual_rows: Some(95),
        execution_time_ms: Some(15),
        metadata: HashMap::new(),
        children: vec![filter_node.id],
    };

    let edges = vec![
        QueryPlanEdge {
            id: Uuid::new_v4(),
            source: scan_node.id,
            target: filter_node.id,
            edge_type: "data_flow".to_string(),
            metadata: HashMap::new(),
        },
        QueryPlanEdge {
            id: Uuid::new_v4(),
            source: filter_node.id,
            target: aggregate_node.id,
            edge_type: "data_flow".to_string(),
            metadata: HashMap::new(),
        },
    ];

    Ok(QueryPlanGraph {
        id: Uuid::new_v4(),
        query_id: Uuid::new_v4(),
        name: "Sample Query Plan".to_string(),
        description: "A sample query plan for demonstration".to_string(),
        nodes: vec![scan_node, filter_node, aggregate_node],
        edges,
        total_estimated_cost: 175.0,
        total_actual_cost: Some(162.0),
        total_execution_time_ms: Some(90),
        metadata: HashMap::new(),
        created_at: Utc::now(),
    })
}

/// Create sample time series data
async fn create_sample_time_series_data() -> BridgeResult<TimeSeriesData> {
    let mut points = Vec::new();
    let now = Utc::now();

    // Create 100 data points with some trend and noise
    for i in 0..100 {
        let timestamp = now - chrono::Duration::minutes((99 - i) as i64);
        let base_value = 100.0 + (i as f64 * 0.5); // Increasing trend
        let noise = (i as f64 * 0.1).sin() * 10.0; // Some noise
        let value = base_value + noise;

        points.push(TimeSeriesPoint {
            timestamp,
            value,
            metadata: HashMap::new(),
        });
    }

    Ok(TimeSeriesData {
        id: Uuid::new_v4(),
        name: "service_response_time".to_string(),
        points,
        metadata: HashMap::new(),
    })
}

/// Get file extension for visualization format
fn get_format_extension(format: &VisualizationFormat) -> &'static str {
    match format {
        VisualizationFormat::Dot => "dot",
        VisualizationFormat::Json => "json",
        VisualizationFormat::Html => "html",
        VisualizationFormat::Svg => "svg",
        VisualizationFormat::Png => "png",
        VisualizationFormat::Pdf => "pdf",
    }
}

/// Get format name for display
fn format_name(format: &VisualizationFormat) -> &'static str {
    match format {
        VisualizationFormat::Dot => "DOT",
        VisualizationFormat::Json => "JSON",
        VisualizationFormat::Html => "HTML",
        VisualizationFormat::Svg => "SVG",
        VisualizationFormat::Png => "PNG",
        VisualizationFormat::Pdf => "PDF",
    }
}
