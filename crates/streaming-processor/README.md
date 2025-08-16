# Streaming Processor

A comprehensive streaming data processor for the bridge, providing real-time data processing capabilities including sources, processors, sinks, windowing, aggregation, and transformation.

## Features

### Data Sources
- **Kafka Source**: Consume data from Kafka topics
- **File Source**: Read data from files (JSON, CSV, Parquet, Avro, Arrow)
- **HTTP Source**: Poll data from HTTP endpoints

### Data Processors
- **Stream Processor**: Basic streaming data processing
- **Filter Processor**: Filter data based on configurable rules
- **Transform Processor**: Transform data using various operations
- **Aggregate Processor**: Aggregate data using windowing and functions

### Data Sinks
- **Kafka Sink**: Produce data to Kafka topics
- **File Sink**: Write data to files in various formats
- **HTTP Sink**: Send data to HTTP endpoints

### Windowing
- **Time Windows**: Process data within time-based windows
- **Count Windows**: Process data within count-based windows
- **Session Windows**: Process data within session-based windows

### Aggregations
- **Count**: Count the number of records
- **Sum**: Sum numeric values
- **Average**: Calculate average of numeric values
- **Min/Max**: Find minimum and maximum values

### Transformations
- **String Operations**: Uppercase, lowercase, trim, replace, substring
- **Field Operations**: Copy, rename, set, remove, add fields

### State Management
- **In-Memory State Store**: Store state in memory
- **State Manager**: Manage multiple state stores

### Metrics Collection
- **In-Memory Metrics Collector**: Collect metrics in memory
- **Metrics Manager**: Manage multiple metrics collectors

## Quick Start

### Basic Usage

```rust
use streaming_processor::{
    sources::{KafkaSource, SourceManager, SourceFactory},
    processors::{StreamProcessor, ProcessorPipeline, ProcessorFactory},
    sinks::{KafkaSink, SinkManager, SinkFactory},
    init_streaming_processor,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize streaming processor
    init_streaming_processor().await?;
    
    // Create source manager
    let mut source_manager = SourceManager::new();
    
    // Configure Kafka source
    let kafka_source_config = sources::KafkaSourceConfig::new(
        vec!["localhost:9092".to_string()],
        "input-topic".to_string(),
        "processor-group".to_string(),
    );
    let kafka_source = SourceFactory::create_source(&kafka_source_config).await?;
    source_manager.add_source("kafka".to_string(), kafka_source);
    
    // Create processor pipeline
    let mut processor_pipeline = ProcessorPipeline::new();
    
    // Add stream processor
    let stream_config = processors::StreamProcessorConfig::new();
    let stream_processor = ProcessorFactory::create_processor(&stream_config).await?;
    processor_pipeline.add_processor(stream_processor);
    
    // Create sink manager
    let mut sink_manager = SinkManager::new();
    
    // Configure Kafka sink
    let kafka_sink_config = sinks::KafkaSinkConfig::new(
        vec!["localhost:9092".to_string()],
        "output-topic".to_string(),
    );
    let kafka_sink = SinkFactory::create_sink(&kafka_sink_config).await?;
    sink_manager.add_sink("kafka".to_string(), kafka_sink);
    
    // Start processing
    source_manager.start_all().await?;
    processor_pipeline.start().await?;
    sink_manager.start_all().await?;
    
    // Main processing loop
    loop {
        // Process data
        // ...
    }
}
```

### Advanced Usage with Filtering and Transformation

```rust
use streaming_processor::processors::{FilterProcessor, TransformProcessor};

// Configure filter processor
let filter_rules = vec![
    processors::FilterRule {
        name: "exclude_debug".to_string(),
        field: "severity".to_string(),
        operator: processors::FilterOperator::NotEquals,
        value: "debug".to_string(),
        enabled: true,
    }
];
let filter_config = processors::FilterProcessorConfig::new(filter_rules, processors::FilterMode::Include);
let filter_processor = ProcessorFactory::create_processor(&filter_config).await?;
processor_pipeline.add_processor(filter_processor);

// Configure transform processor
let transform_rules = vec![
    processors::TransformRule {
        name: "add_timestamp".to_string(),
        rule_type: processors::TransformRuleType::Add,
        source_field: "".to_string(),
        target_field: "processing_timestamp".to_string(),
        transform_value: Some("${timestamp}".to_string()),
        enabled: true,
    }
];
let transform_config = processors::TransformProcessorConfig::new(transform_rules);
let transform_processor = ProcessorFactory::create_processor(&transform_config).await?;
processor_pipeline.add_processor(transform_processor);
```

### Windowing and Aggregation

```rust
use streaming_processor::{
    windows::{TimeWindow, WindowManager},
    aggregations::{AggregationManager, AggregationFactory},
};

// Create window manager
let mut window_manager = WindowManager::new();

// Create time window
let time_window = TimeWindow::new(
    "time-window-1".to_string(),
    chrono::Utc::now(),
    chrono::Duration::minutes(5),
);
window_manager.add_window(Box::new(time_window));

// Create aggregation manager
let aggregation_manager = AggregationManager::new();

// Use aggregation functions
let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
let sum = aggregation_manager.apply_function("sum", values).await?;
let average = aggregation_manager.apply_function("average", values).await?;
```

## Architecture

The streaming processor follows a modular architecture with the following components:

### Sources
- **KafkaSource**: Consumes data from Kafka topics
- **FileSource**: Reads data from files
- **HttpSource**: Polls data from HTTP endpoints

### Processors
- **StreamProcessor**: Basic streaming processing
- **FilterProcessor**: Filters data based on rules
- **TransformProcessor**: Transforms data using operations
- **AggregateProcessor**: Aggregates data using functions

### Sinks
- **KafkaSink**: Produces data to Kafka topics
- **FileSink**: Writes data to files
- **HttpSink**: Sends data to HTTP endpoints

### Windowing
- **TimeWindow**: Time-based windows
- **CountWindow**: Count-based windows
- **SessionWindow**: Session-based windows

### State Management
- **InMemoryStateStore**: In-memory state storage
- **StateManager**: Manages multiple state stores

### Metrics Collection
- **InMemoryMetricsCollector**: In-memory metrics collection
- **MetricsManager**: Manages multiple metrics collectors

## Configuration

### Source Configuration

```rust
// Kafka Source
let kafka_config = sources::KafkaSourceConfig::new(
    vec!["localhost:9092".to_string()],
    "topic".to_string(),
    "group".to_string(),
);

// File Source
let file_config = sources::FileSourceConfig::new(
    "/path/to/file.json".to_string(),
    sources::FileFormat::Json,
);

// HTTP Source
let http_config = sources::HttpSourceConfig::new(
    "http://localhost:8080/data".to_string(),
);
```

### Processor Configuration

```rust
// Stream Processor
let stream_config = processors::StreamProcessorConfig::new();

// Filter Processor
let filter_config = processors::FilterProcessorConfig::new(
    filter_rules,
    processors::FilterMode::Include,
);

// Transform Processor
let transform_config = processors::TransformProcessorConfig::new(transform_rules);

// Aggregate Processor
let aggregate_config = processors::AggregateProcessorConfig::new(
    aggregation_rules,
    60000, // window size in milliseconds
);
```

### Sink Configuration

```rust
// Kafka Sink
let kafka_sink_config = sinks::KafkaSinkConfig::new(
    vec!["localhost:9092".to_string()],
    "topic".to_string(),
);

// File Sink
let file_sink_config = sinks::FileSinkConfig::new(
    "/path/to/output.json".to_string(),
    sinks::FileFormat::Json,
);

// HTTP Sink
let http_sink_config = sinks::HttpSinkConfig::new(
    "http://localhost:8081/data".to_string(),
);
```

## Examples

See the `examples/` directory for complete examples:

- `basic_streaming.rs`: Basic streaming processor usage
- Additional examples coming soon

## Dependencies

- `bridge-core`: Core bridge types and traits
- `tokio`: Async runtime
- `serde`: Serialization/deserialization
- `tracing`: Logging and tracing
- `chrono`: Date and time handling
- `uuid`: UUID generation
- `opentelemetry`: OpenTelemetry support

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Running Examples

```bash
cargo run --example basic_streaming
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

This project is licensed under the Apache 2.0 License - see the LICENSE file for details.
