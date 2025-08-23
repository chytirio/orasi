# Phase 3 Features: Streaming Queries, Query Plan Visualization, and Advanced Analytics

This document describes the new Phase 3 features implemented in the Orasi query engine, building upon the solid DataFusion integration foundation.

## 🚀 Overview

Phase 3 introduces three major feature areas that enhance the query engine's capabilities:

1. **Streaming Queries** - Real-time data processing capabilities
2. **Query Plan Visualization** - Tools for analyzing and optimizing query performance  
3. **Advanced Analytics** - Time series analysis and machine learning integration

These features provide a comprehensive solution for modern observability workloads, enabling real-time analytics, performance optimization, and advanced data insights.

## 📊 Streaming Queries

### Overview

Streaming queries enable real-time data processing by executing queries continuously as new data arrives. This is essential for monitoring, alerting, and real-time analytics use cases.

### Key Features

- **Continuous Query Execution**: Queries that run continuously and process data as it arrives
- **Windowing Support**: Time-based, count-based, and session windows for data aggregation
- **Backpressure Handling**: Automatic backpressure management to prevent system overload
- **Result Streaming**: Real-time streaming of query results to downstream consumers
- **Query Management**: Centralized management of multiple streaming queries

### Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Data Sources  │───▶│ Streaming Query │───▶│ Result Streams  │
│                 │    │   Manager       │    │                 │
│ • Kafka         │    │                 │    │ • WebSockets    │
│ • OTLP          │    │ • Query Registry│    │ • gRPC Streams  │
│ • HTTP          │    │ • Window Mgmt   │    │ • Event Bus     │
│ • Files         │    │ • Backpressure  │    │ • Message Queues│
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Usage Example

```rust
use query_engine::{
    StreamingQueryConfig, ContinuousQuery, ContinuousQueryConfig,
    WindowConfig, WindowType, ContinuousQueryManager,
};

// Create streaming query configuration
let streaming_config = StreamingQueryConfig::default();

// Create continuous query configuration
let continuous_config = ContinuousQueryConfig {
    name: "service_latency_monitor".to_string(),
    sql: "SELECT service_name, AVG(response_time) as avg_latency 
          FROM telemetry 
          WHERE timestamp >= NOW() - INTERVAL '1 hour' 
          GROUP BY service_name".to_string(),
    execution_interval_ms: 5000, // Execute every 5 seconds
    window_config: Some(WindowConfig {
        window_type: WindowType::Time,
        window_size: 60000, // 1 minute window
        window_slide: Some(30000), // 30 second slide
        enable_watermarking: true,
        watermark_delay: Some(5000),
        ..Default::default()
    }),
    enable_result_streaming: true,
    result_buffer_size: 1000,
    ..Default::default()
};

// Create and start continuous query
let mut continuous_query = ContinuousQuery::new(
    continuous_config,
    streaming_config,
    executor,
);

continuous_query.start().await?;

// Manage queries with manager
let mut query_manager = ContinuousQueryManager::new();
query_manager.register_query(Box::new(continuous_query)).await?;
```

### Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `enable_streaming` | Enable streaming queries | `true` |
| `max_concurrent_queries` | Maximum concurrent streaming queries | `100` |
| `default_window_ms` | Default window size in milliseconds | `60000` |
| `enable_backpressure` | Enable backpressure handling | `true` |
| `backpressure_threshold` | Backpressure threshold (percentage) | `80` |
| `enable_caching` | Enable query result caching | `true` |
| `cache_ttl_secs` | Cache TTL in seconds | `300` |

## 📈 Query Plan Visualization

### Overview

Query plan visualization provides tools for analyzing and optimizing query performance by generating visual representations of query execution plans.

### Key Features

- **Multiple Output Formats**: DOT, JSON, HTML, SVG, PNG, PDF
- **Performance Annotations**: Cost estimates, execution times, row counts
- **Color Coding**: Visual distinction between different node types
- **Interactive HTML**: Rich HTML output with detailed node information
- **Export Capabilities**: Save visualizations to files for sharing and analysis

### Supported Node Types

- **TableScan**: Data source scanning operations
- **Filter**: WHERE clause filtering
- **Join**: Table join operations (Hash, Merge, Nested Loop)
- **Aggregate**: GROUP BY and aggregation operations
- **Sort**: ORDER BY operations
- **Project**: Column selection and projection
- **Window**: Window function operations

### Usage Example

```rust
use query_engine::{
    QueryPlanGraph, PlanVisualizer, PlanVisualizerConfig,
    VisualizationFormat, Visualizer,
};

// Create plan visualizer configuration
let visualizer_config = PlanVisualizerConfig {
    name: "performance_analyzer".to_string(),
    version: "1.0.0".to_string(),
    enable_visualization: true,
    default_format: VisualizationFormat::Dot,
    output_directory: "./visualizations".to_string(),
    enable_color_coding: true,
    enable_cost_annotations: true,
    enable_performance_annotations: true,
    ..Default::default()
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
    let path = format!("./visualizations/query_plan.{}", get_extension(&format));
    visualizer.save_visualization(&query_plan, &format, &path).await?;
}
```

### Visualization Formats

| Format | Description | Use Case |
|--------|-------------|----------|
| **DOT** | Graphviz DOT format | Integration with Graphviz tools |
| **JSON** | Structured data format | API integration, programmatic analysis |
| **HTML** | Interactive web format | Web dashboards, documentation |
| **SVG** | Scalable vector graphics | High-quality printing, web embedding |
| **PNG** | Raster image format | Documentation, presentations |
| **PDF** | Portable document format | Reports, documentation |

### Performance Metrics

The visualization includes comprehensive performance metrics:

- **Estimated vs Actual Cost**: Compare optimizer estimates with actual execution
- **Row Counts**: Estimated and actual row counts for each operation
- **Execution Times**: Per-node execution timing information
- **Resource Usage**: CPU, memory, and I/O metrics
- **Cache Hit Ratios**: Query cache effectiveness

## 🔬 Advanced Analytics

### Overview

Advanced analytics provides sophisticated analysis capabilities including time series analysis, anomaly detection, and machine learning integration.

### Key Features

- **Time Series Analysis**: Trend detection, seasonality analysis, statistical summaries
- **Anomaly Detection**: Point, contextual, collective, and trend anomaly detection
- **Machine Learning Integration**: Predictive modeling and forecasting
- **Clustering Analysis**: Pattern discovery and data segmentation
- **Real-time Processing**: Streaming analytics on live data

### Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Time Series   │───▶│   Analytics     │───▶│   Results       │
│     Data        │    │   Engine        │    │                 │
│                 │    │                 │    │ • Anomalies     │
│ • Metrics       │    │ • Trend Analysis│    │ • Predictions   │
│ • Traces        │    │ • Seasonality   │    │ • Clusters      │
│ • Logs          │    │ • Anomaly Det.  │    │ • Insights      │
│ • Events        │    │ • ML Models     │    │ • Alerts        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Time Series Analysis

#### Statistical Analysis

```rust
use query_engine::{
    TimeSeriesData, TimeSeriesAnalyzer, TimeSeriesAnalyzerConfig,
    TimeSeriesResult,
};

// Create time series analyzer
let analyzer_config = TimeSeriesAnalyzerConfig::default();
let analyzer = TimeSeriesAnalyzer::new(analyzer_config);

// Analyze time series data
let result = analyzer.analyze_time_series(&data).await?;

println!("Statistics:");
println!("  - Data points: {}", result.statistics.count);
println!("  - Mean: {:.2}", result.statistics.mean);
println!("  - Std Dev: {:.2}", result.statistics.std_dev);
println!("  - Min: {:.2}", result.statistics.min);
println!("  - Max: {:.2}", result.statistics.max);
```

#### Trend Analysis

```rust
if let Some(trend) = &result.trend_analysis {
    println!("Trend Analysis:");
    println!("  - Direction: {:?}", trend.direction);
    println!("  - Slope: {:.4}", trend.slope);
    println!("  - Strength: {:.2}", trend.strength);
    println!("  - Confidence: {:.2}", trend.confidence);
}
```

#### Seasonality Analysis

```rust
if let Some(seasonality) = &result.seasonality_analysis {
    println!("Seasonality Analysis:");
    println!("  - Period: {} seconds", seasonality.period_secs);
    println!("  - Strength: {:.2}", seasonality.strength);
    println!("  - Confidence: {:.2}", seasonality.confidence);
}
```

### Anomaly Detection

#### Supported Anomaly Types

- **Point Anomalies**: Individual data points that are anomalous
- **Contextual Anomalies**: Data points anomalous in their context
- **Collective Anomalies**: Groups of data points that are anomalous together
- **Trend Anomalies**: Changes in trend patterns
- **Seasonal Anomalies**: Deviations from seasonal patterns

#### Usage Example

```rust
use query_engine::{
    AnomalyDetector, AnomalyDetectorConfig, AnomalyResult, AnomalyType,
};

// Create anomaly detector
let detector_config = AnomalyDetectorConfig {
    name: "performance_monitor".to_string(),
    version: "1.0.0".to_string(),
    enable_detection: true,
    threshold: 2.0, // 2 standard deviations
    window_size: 100,
    ..Default::default()
};

let detector = DefaultAnomalyDetector::new(detector_config);

// Detect anomalies
let anomalies = detector.detect_anomalies(&data).await?;

for anomaly in &anomalies {
    println!("Anomaly: {:?} at {} (confidence: {:.2})", 
        anomaly.anomaly_type, anomaly.timestamp, anomaly.confidence);
}
```

### Machine Learning Integration

#### Supported ML Capabilities

- **Time Series Forecasting**: ARIMA, Exponential Smoothing, Prophet, LSTM
- **Anomaly Detection**: Isolation Forest, One-Class SVM, Autoencoders
- **Clustering**: K-means, DBSCAN, Hierarchical, Spectral clustering
- **Regression**: Linear regression, Random Forest, Neural Networks
- **Classification**: Support Vector Machines, Random Forest, Neural Networks

#### Usage Example

```rust
use query_engine::{
    MLPredictor, MLPredictorConfig, PredictionResult,
};

// Create ML predictor
let predictor_config = MLPredictorConfig {
    name: "forecast_predictor".to_string(),
    version: "1.0.0".to_string(),
    enable_prediction: true,
    models: vec![], // Configure ML models
    ..Default::default()
};

let predictor = DefaultMLPredictor::new(predictor_config);

// Make predictions
let predictions = predictor.predict(&data, 10).await?; // 10 steps ahead

for prediction in &predictions {
    println!("Prediction: {:.2} (confidence: {:.2})", 
        prediction.predicted_value, prediction.confidence);
}
```

## 🛠️ Configuration

### Streaming Queries Configuration

```toml
[streaming]
enable_streaming = true
max_concurrent_queries = 100
default_window_ms = 60000
enable_backpressure = true
backpressure_threshold = 80
enable_caching = true
cache_ttl_secs = 300
enable_result_streaming = true
max_result_buffer_size = 10000
```

### Query Plan Visualization Configuration

```toml
[visualization]
enable_plan_visualization = true
enable_performance_analysis = true
enable_metrics_collection = true
enable_report_generation = true
default_format = "dot"
output_directory = "./visualizations"
```

### Advanced Analytics Configuration

```toml
[analytics]
enable_time_series = true
enable_anomaly_detection = true
enable_ml_integration = true
enable_forecasting = true
enable_clustering = true
default_analysis_window_secs = 3600
confidence_threshold = 0.8
```

## 📋 Example Usage

### Complete Example

See `examples/streaming_queries_example.rs` for a complete demonstration of all Phase 3 features:

```bash
# Run the example
cargo run --example streaming_queries_example
```

This example demonstrates:

1. **Streaming Queries**: Setting up continuous queries with windowing
2. **Query Plan Visualization**: Generating visualizations in multiple formats
3. **Advanced Analytics**: Time series analysis, anomaly detection, and ML integration

### Key Components Demonstrated

- Continuous query execution with real-time monitoring
- Query plan visualization with performance metrics
- Time series analysis with trend and seasonality detection
- Anomaly detection with confidence scoring
- Machine learning prediction capabilities

## 🔧 Integration with Existing Features

### DataFusion Integration

All Phase 3 features build upon the existing DataFusion integration:

- **Streaming Queries**: Use DataFusion for query execution
- **Query Plan Visualization**: Visualize DataFusion query plans
- **Advanced Analytics**: Process DataFusion query results

### OpenTelemetry Integration

Seamless integration with OpenTelemetry data:

- **Streaming Queries**: Process OTLP streams in real-time
- **Query Plan Visualization**: Analyze query performance on telemetry data
- **Advanced Analytics**: Analyze metrics, traces, and logs

### Lakehouse Integration

Full compatibility with lakehouse storage:

- **Streaming Queries**: Query Delta Lake, Iceberg, and other formats
- **Query Plan Visualization**: Optimize lakehouse queries
- **Advanced Analytics**: Analyze lakehouse data with ML capabilities

## 🚀 Performance Considerations

### Streaming Queries

- **Memory Management**: Automatic memory management with configurable limits
- **Backpressure**: Built-in backpressure handling to prevent system overload
- **Parallelism**: Configurable parallelism for query execution
- **Caching**: Intelligent caching to reduce redundant computations

### Query Plan Visualization

- **Lazy Generation**: Visualizations generated on-demand
- **Caching**: Cached visualizations for repeated access
- **Compression**: Efficient storage of visualization data
- **Streaming**: Large visualizations streamed to avoid memory issues

### Advanced Analytics

- **Incremental Processing**: Process data incrementally to handle large datasets
- **Parallel Algorithms**: Parallel execution of analytics algorithms
- **Memory Optimization**: Efficient memory usage for large time series
- **Caching**: Cache intermediate results for repeated analysis

## 🔮 Future Enhancements

### Planned Features

1. **Advanced Windowing**: More sophisticated windowing functions
2. **Real-time ML**: Online machine learning capabilities
3. **Interactive Visualizations**: Web-based interactive query plan explorer
4. **Advanced Anomaly Detection**: Deep learning-based anomaly detection
5. **Predictive Analytics**: Advanced forecasting and prediction capabilities

### Integration Roadmap

1. **Grafana Integration**: Direct integration with Grafana dashboards
2. **Kafka Integration**: Native Kafka streaming support
3. **Kubernetes Integration**: Kubernetes-native deployment and scaling
4. **Cloud Integration**: AWS, GCP, and Azure native integrations

## 📚 Additional Resources

- [API Reference](API_REFERENCE.md)
- [Configuration Guide](CONFIGURATION.md)
- [Deployment Guide](DEPLOYMENT.md)
- [Performance Tuning](PERFORMANCE.md)
- [Troubleshooting](TROUBLESHOOTING.md)

## 🤝 Contributing

We welcome contributions to enhance Phase 3 features! Please see our [Contributing Guide](CONTRIBUTING.md) for details on how to get started.

## 📄 License

This project is licensed under the Apache License 2.0 - see the [LICENSE](../LICENSE) file for details.
