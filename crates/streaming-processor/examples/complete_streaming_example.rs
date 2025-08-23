//! Complete streaming processor example
//!
//! This example demonstrates a complete streaming processor setup with:
//! - Multiple data sources (file, HTTP, Kafka)
//! - Data processing pipeline (filtering, transformation, aggregation)
//! - Multiple output sinks (file, console, HTTP)
//! - Real-time data processing

use std::collections::HashMap;
use std::time::Duration;
use streaming_processor::{
    config::{
        ProcessorConfig, ProcessorType, SinkConfig, SinkType, SourceConfig, SourceType,
        StreamingProcessorConfig,
    },
    StreamingProcessor,
};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting complete streaming processor example");

    // Create a comprehensive configuration
    let config = create_complete_config();

    // Create and start the streaming processor
    let mut processor = StreamingProcessor::new(config).await?;

    info!("Initializing streaming processor...");
    processor.start().await?;

    info!("Streaming processor started successfully!");
    info!("Press Ctrl+C to stop...");

    // Keep the processor running
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        // Print statistics every 5 seconds
        let stats = processor.get_stats().await;
        info!("Processor stats: {:?}", stats);

        // Check if processor is still running
        if !processor.is_running().await {
            warn!("Processor stopped unexpectedly");
            break;
        }
    }

    // Stop the processor
    info!("Stopping streaming processor...");
    processor.stop().await?;
    info!("Streaming processor stopped successfully");

    Ok(())
}

fn create_complete_config() -> StreamingProcessorConfig {
    let mut sources = HashMap::new();
    let mut sinks = HashMap::new();

    // File source configuration
    let mut file_source_config = HashMap::new();
    file_source_config.insert(
        "file_path".to_string(),
        serde_json::Value::String("/tmp/input.json".to_string()),
    );
    file_source_config.insert(
        "format".to_string(),
        serde_json::Value::String("json".to_string()),
    );
    file_source_config.insert(
        "enable_file_watching".to_string(),
        serde_json::Value::Bool(true),
    );

    sources.insert(
        "file-source".to_string(),
        SourceConfig {
            source_type: SourceType::File,
            name: "file-source".to_string(),
            version: "1.0.0".to_string(),
            config: file_source_config,
            auth: None,
            connection: streaming_processor::config::ConnectionConfig::default(),
        },
    );

    // HTTP source configuration
    let mut http_source_config = HashMap::new();
    http_source_config.insert(
        "endpoint".to_string(),
        serde_json::Value::String("http://localhost:8080/metrics".to_string()),
    );
    http_source_config.insert(
        "polling_interval_ms".to_string(),
        serde_json::Value::Number(serde_json::Number::from(5000)),
    );

    sources.insert(
        "http-source".to_string(),
        SourceConfig {
            source_type: SourceType::Http,
            name: "http-source".to_string(),
            version: "1.0.0".to_string(),
            config: http_source_config,
            auth: None,
            connection: streaming_processor::config::ConnectionConfig::default(),
        },
    );

    // Kafka source configuration
    let mut kafka_source_config = HashMap::new();
    kafka_source_config.insert(
        "endpoint".to_string(),
        serde_json::Value::String("localhost:9092".to_string()),
    );
    kafka_source_config.insert(
        "topic".to_string(),
        serde_json::Value::String("telemetry-data".to_string()),
    );

    sources.insert(
        "kafka-source".to_string(),
        SourceConfig {
            source_type: SourceType::Kafka,
            name: "kafka-source".to_string(),
            version: "1.0.0".to_string(),
            config: kafka_source_config,
            auth: None,
            connection: streaming_processor::config::ConnectionConfig::default(),
        },
    );

    // Processors configuration
    let processors = vec![
        // Filter processor
        ProcessorConfig {
            processor_type: ProcessorType::Filter,
            name: "error-filter".to_string(),
            version: "1.0.0".to_string(),
            config: {
                let mut config = HashMap::new();
                let mut conditions = HashMap::new();
                conditions.insert(
                    "level".to_string(),
                    serde_json::Value::String("error".to_string()),
                );
                config.insert(
                    "conditions".to_string(),
                    serde_json::Value::Object(serde_json::Map::from_iter(
                        conditions.into_iter().map(|(k, v)| (k, v)),
                    )),
                );
                config
            },
            order: 1,
        },
        // Transform processor
        ProcessorConfig {
            processor_type: ProcessorType::Transform,
            name: "uppercase-transform".to_string(),
            version: "1.0.0".to_string(),
            config: {
                let mut config = HashMap::new();
                config.insert(
                    "transformations".to_string(),
                    serde_json::Value::Array(vec![
                        serde_json::Value::String("uppercase".to_string()),
                        serde_json::Value::String("trim".to_string()),
                    ]),
                );
                config
            },
            order: 2,
        },
        // Aggregate processor
        ProcessorConfig {
            processor_type: ProcessorType::Aggregate,
            name: "metrics-aggregator".to_string(),
            version: "1.0.0".to_string(),
            config: {
                let mut config = HashMap::new();
                config.insert(
                    "aggregations".to_string(),
                    serde_json::Value::Array(vec![
                        serde_json::Value::String("sum".to_string()),
                        serde_json::Value::String("average".to_string()),
                        serde_json::Value::String("count".to_string()),
                    ]),
                );
                config
            },
            order: 3,
        },
        // Window processor
        ProcessorConfig {
            processor_type: ProcessorType::Window,
            name: "time-window".to_string(),
            version: "1.0.0".to_string(),
            config: {
                let mut config = HashMap::new();
                let mut window_config = HashMap::new();
                window_config.insert(
                    "window_type".to_string(),
                    serde_json::Value::String("time".to_string()),
                );
                window_config.insert(
                    "duration_seconds".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(60)),
                );
                config.insert(
                    "window_config".to_string(),
                    serde_json::Value::Object(serde_json::Map::from_iter(
                        window_config.into_iter().map(|(k, v)| (k, v)),
                    )),
                );
                config
            },
            order: 4,
        },
    ];

    // File sink configuration
    let mut file_sink_config = HashMap::new();
    file_sink_config.insert(
        "file_path".to_string(),
        serde_json::Value::String("/tmp/processed_output.json".to_string()),
    );

    sinks.insert(
        "file-sink".to_string(),
        SinkConfig {
            sink_type: SinkType::File,
            name: "file-sink".to_string(),
            version: "1.0.0".to_string(),
            config: file_sink_config,
            auth: None,
            connection: streaming_processor::config::ConnectionConfig::default(),
        },
    );

    // HTTP sink configuration
    let mut http_sink_config = HashMap::new();
    http_sink_config.insert(
        "endpoint".to_string(),
        serde_json::Value::String("http://localhost:8081/processed".to_string()),
    );

    sinks.insert(
        "http-sink".to_string(),
        SinkConfig {
            sink_type: SinkType::Http,
            name: "http-sink".to_string(),
            version: "1.0.0".to_string(),
            config: http_sink_config,
            auth: None,
            connection: streaming_processor::config::ConnectionConfig::default(),
        },
    );

    StreamingProcessorConfig {
        name: "complete-streaming-processor".to_string(),
        version: "1.0.0".to_string(),
        sources,
        processors,
        sinks,
        processing: streaming_processor::config::ProcessingConfig {
            batch_size: 1000,
            buffer_size: 10000,
            timeout: Duration::from_secs(30),
            enable_parallel: true,
            num_workers: 4,
            enable_backpressure: true,
            backpressure_threshold: 80,
        },
        state: streaming_processor::config::StateConfig::default(),
        metrics: streaming_processor::config::MetricsConfig {
            enable_metrics: true,
            endpoint: Some("0.0.0.0:9090".to_string()),
            collection_interval: Duration::from_secs(15),
            enable_health_checks: true,
            health_check_interval: Duration::from_secs(30),
        },
        security: streaming_processor::config::SecurityConfig::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_config_creation() {
        let config = create_complete_config();
        assert_eq!(config.name, "complete-streaming-processor");
        assert_eq!(config.sources.len(), 3);
        assert_eq!(config.processors.len(), 4);
        assert_eq!(config.sinks.len(), 2);
    }

    #[tokio::test]
    async fn test_config_validation() {
        let config = create_complete_config();
        assert!(config.validate().is_ok());
    }
}
