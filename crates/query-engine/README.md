# Query Engine

A high-performance query engine for the Orasi bridge system, providing SQL query execution, analytics, and machine learning capabilities.

## Features

- **SQL Query Execution**: Execute SQL queries against various data sources
- **Multi-Source Support**: Delta Lake, Iceberg, S3 Parquet, and more
- **Advanced Analytics**: Time series analysis, anomaly detection, clustering
- **Machine Learning Integration**: Statistical models and prediction capabilities
- **Performance Optimization**: Query optimization and caching
- **Real-time Streaming**: Continuous query processing
- **Visualization**: Metrics collection and reporting

## Machine Learning Integration

The query engine includes comprehensive machine learning capabilities for time series prediction and analysis:

### Statistical Models

The ML integration provides several statistical models for time series prediction:

- **Moving Average**: Simple moving average with configurable window size
- **Linear Regression**: Trend-based predictions using linear regression
- **Exponential Smoothing**: Smoothing-based predictions with configurable alpha
- **ARIMA**: Simplified ARIMA-like model for time series forecasting

### Features

- **Ensemble Predictions**: Combines multiple models for improved accuracy
- **Confidence Intervals**: Provides confidence bounds for predictions
- **Configurable Models**: Customize model parameters through configuration
- **Error Handling**: Graceful handling of edge cases and errors
- **Extensible Architecture**: Easy to add new model types

## Anomaly Detection

The query engine provides advanced anomaly detection capabilities for identifying unusual patterns in time series data:

### Detection Methods

The anomaly detection system supports multiple statistical methods:

- **Z-Score Detection**: Identifies outliers based on standard deviations from the mean
- **Moving Average Detection**: Detects anomalies relative to local moving averages
- **IQR Detection**: Uses interquartile range to identify statistical outliers
- **Trend Detection**: Identifies anomalies in trend patterns
- **Seasonal Detection**: Detects anomalies in seasonal patterns

### Anomaly Types

The system can identify various types of anomalies:

- **Point Anomalies**: Individual data points that are statistically unusual
- **Contextual Anomalies**: Values that are normal in isolation but anomalous in context
- **Collective Anomalies**: Groups of data points that are anomalous together
- **Trend Anomalies**: Changes in trend patterns that are unusual
- **Seasonal Anomalies**: Breaks in expected seasonal patterns

### Features

- **Multi-Method Detection**: Combines multiple detection methods for comprehensive coverage
- **Confidence Scoring**: Provides confidence levels for each detected anomaly
- **Severity Assessment**: Calculates severity scores based on deviation magnitude
- **Configurable Thresholds**: Customize sensitivity for different detection methods
- **Rich Metadata**: Includes detailed information about each detected anomaly

### ML Prediction Usage Example

```rust
use query_engine::analytics::{
    ml_integration::{DefaultMLPredictor, MLPredictorConfig, MLPredictor},
    TimeSeriesData, TimeSeriesPoint,
};

// Create ML predictor configuration
let config = MLPredictorConfig {
    name: "my_predictor".to_string(),
    version: "1.0.0".to_string(),
    enable_prediction: true,
    models: vec![], // Use default models
    additional_config: HashMap::new(),
};

// Create predictor and make predictions
let predictor = DefaultMLPredictor::new(config);
let predictions = predictor.predict(&time_series_data, 10).await?;

// Access prediction results
for prediction in predictions {
    println!(
        "Predicted: {:.2} (confidence: {:.1}%)",
        prediction.predicted_value,
        prediction.confidence * 100.0
    );
}
```

### Anomaly Detection Usage Example

```rust
use query_engine::analytics::{
    anomaly_detection::{DefaultAnomalyDetector, AnomalyDetectorConfig, AnomalyDetector},
    TimeSeriesData, TimeSeriesPoint,
};

// Create anomaly detector configuration
let config = AnomalyDetectorConfig {
    name: "my_detector".to_string(),
    version: "1.0.0".to_string(),
    enable_detection: true,
    threshold: 2.0,
    window_size: 10,
    additional_config: HashMap::new(),
};

// Create detector and find anomalies
let detector = DefaultAnomalyDetector::new(config);
let anomalies = detector.detect_anomalies(&time_series_data).await?;

// Access anomaly results
for anomaly in anomalies {
    println!(
        "Anomaly: {:?} (confidence: {:.1}%, severity: {:.1}%)",
        anomaly.anomaly_type,
        anomaly.confidence * 100.0,
        anomaly.severity * 100.0
    );
}
```

### Custom Model Configuration

You can configure specific models with custom parameters:

```rust
let custom_config = MLPredictorConfig {
    name: "custom_predictor".to_string(),
    version: "1.0.0".to_string(),
    enable_prediction: true,
    models: vec![
        MLModelConfig {
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
        MLModelConfig {
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
```

### Running the Examples

To see the ML prediction functionality in action:

```bash
cargo run --example ml_prediction_example
```

This will demonstrate:
- Time series data generation with trends and noise
- Ensemble predictions using multiple statistical models
- Confidence interval calculation
- Custom model configuration
- Error handling for edge cases

To see the anomaly detection functionality in action:

```bash
cargo run --example anomaly_detection_example
```

This will demonstrate:
- Time series data generation with various anomaly types
- Multi-method anomaly detection (Z-score, IQR, moving average, etc.)
- Confidence and severity scoring
- Custom detection method configuration
- Anomaly statistics and analysis
- Error handling for edge cases

## Installation

```bash
cargo add query-engine
```

## Quick Start

```rust
use query_engine::QueryEngine;

let engine = QueryEngine::new(config).await?;
let result = engine.execute_query("SELECT * FROM my_table").await?;
```

## Documentation

For detailed documentation, see the [API Reference](docs/API_REFERENCE.md).

## Contributing

Please read [CONTRIBUTING.md](../../CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](../../LICENSE) file for details.
