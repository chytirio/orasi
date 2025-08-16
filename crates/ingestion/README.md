# OpenTelemetry Ingestion Crate

This crate provides comprehensive OpenTelemetry data ingestion capabilities for the Orasi bridge, supporting multiple protocols and data formats.

## Features

### Supported Protocols

- **OTLP Arrow**: Ingest OpenTelemetry data in Arrow format over HTTP/gRPC
- **OTLP gRPC**: Standard OpenTelemetry protocol over gRPC
- **Kafka**: Ingest telemetry data from Kafka topics with support for various serialization formats

### Data Processing

- **Batch Processing**: Efficient batching of telemetry data
- **Filtering**: Filter data based on configurable rules
- **Transformation**: Transform and enrich telemetry data
- **Validation**: Schema and data quality validation

### Data Export

- **Batch Export**: Export data in batches to various destinations
- **Streaming Export**: Real-time streaming of telemetry data

## Quick Start

### Basic Usage

```rust
use ingestion::{
    OtlpReceiver, KafkaReceiver, ReceiverManager, ReceiverFactory,
    receivers::{OtlpReceiverConfig, KafkaReceiverConfig},
    BatchProcessor, ProcessorPipeline, ProcessorFactory,
    processors::BatchProcessorConfig,
    BatchExporter, ExporterPipeline, ExporterFactory,
    exporters::BatchExporterConfig,
    SchemaValidator, DataValidator,
};

#[tokio::main]
async fn main() -> bridge_core::BridgeResult<()> {
    // Create receiver manager
    let mut receiver_manager = ReceiverManager::new();
    
    // Configure OTLP Arrow receiver
    let otlp_config = OtlpReceiverConfig::new_arrow("0.0.0.0".to_string(), 4318);
    let otlp_receiver = ReceiverFactory::create_receiver(&otlp_config).await?;
    receiver_manager.add_receiver("otlp-arrow".to_string(), otlp_receiver);
    
    // Configure Kafka receiver
    let kafka_config = KafkaReceiverConfig::new(
        vec!["localhost:9092".to_string()],
        "telemetry-data".to_string(),
        "ingestion-group".to_string(),
    );
    let kafka_receiver = ReceiverFactory::create_receiver(&kafka_config).await?;
    receiver_manager.add_receiver("kafka".to_string(), kafka_receiver);
    
    // Start receivers
    receiver_manager.start_all().await?;
    
    // Create processor pipeline
    let mut processor_pipeline = ProcessorPipeline::new();
    let batch_config = BatchProcessorConfig::new(1000, 5000);
    let batch_processor = ProcessorFactory::create_processor(&batch_config).await?;
    processor_pipeline.add_processor(batch_processor);
    processor_pipeline.start().await?;
    
    // Create exporter pipeline
    let mut exporter_pipeline = ExporterPipeline::new();
    let exporter_config = BatchExporterConfig::new("http://localhost:8080/export".to_string(), 1000);
    let exporter = ExporterFactory::create_exporter(&exporter_config).await?;
    exporter_pipeline.add_exporter(exporter);
    exporter_pipeline.start().await?;
    
    // Create validators
    let schema_validator = SchemaValidator::new();
    let data_validator = DataValidator::new();
    
    // Main processing loop
    loop {
        for receiver_name in receiver_manager.get_receiver_names() {
            if let Some(receiver) = receiver_manager.get_receiver(&receiver_name) {
                if let Ok(batch) = receiver.receive().await {
                    // Validate
                    if schema_validator.validate(&batch).await?.is_valid {
                        // Process
                        let processed = processor_pipeline.process(batch).await?;
                        // Export
                        let _ = exporter_pipeline.export(processed.batch).await;
                    }
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
```

## Architecture

### Receivers

Receivers handle incoming telemetry data from various sources:

- **OtlpReceiver**: Handles OTLP Arrow and OTLP gRPC protocols
- **KafkaReceiver**: Consumes telemetry data from Kafka topics
- **HttpReceiver**: Receives data over HTTP endpoints

### Processors

Processors transform and enrich telemetry data:

- **BatchProcessor**: Batches data for efficient processing
- **FilterProcessor**: Filters data based on configurable rules
- **TransformProcessor**: Transforms and enriches data

### Exporters

Exporters send processed data to various destinations:

- **BatchExporter**: Exports data in batches
- **StreamingExporter**: Streams data in real-time

### Validators

Validators ensure data quality:

- **SchemaValidator**: Validates data structure and schema
- **DataValidator**: Validates data quality and consistency

## Configuration

### OTLP Receiver Configuration

```rust
let config = OtlpReceiverConfig::new_arrow("0.0.0.0".to_string(), 4318);
```

### Kafka Receiver Configuration

```rust
let config = KafkaReceiverConfig::new(
    vec!["localhost:9092".to_string()],
    "telemetry-data".to_string(),
    "ingestion-group".to_string(),
);
```

### Batch Processor Configuration

```rust
let config = BatchProcessorConfig::new(1000, 5000); // max_batch_size, max_batch_time_ms
```

### Filter Processor Configuration

```rust
let rules = vec![
    FilterRule {
        name: "exclude_debug".to_string(),
        field: "severity".to_string(),
        operator: FilterOperator::NotEquals,
        value: "debug".to_string(),
        enabled: true,
    }
];
let config = FilterProcessorConfig::new(rules, FilterMode::Include);
```

## Examples

See the `examples/` directory for complete working examples:

- `basic_ingestion.rs`: Basic ingestion setup with all protocols
- Additional examples coming soon

## Dependencies

This crate depends on:

- `bridge-core`: Core bridge types and traits
- `tokio`: Async runtime
- `serde`: Serialization
- `tracing`: Logging
- `tonic`: gRPC support
- `opentelemetry-proto`: OpenTelemetry protocol definitions

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
cargo run --example basic_ingestion
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

This project is licensed under the Apache 2.0 License - see the LICENSE file for details.
