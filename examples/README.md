# Orasi Examples

This directory contains examples demonstrating various features and capabilities of the Orasi platform. Examples are organized by functionality to help you find relevant demonstrations quickly.

## Directory Structure

### Core Examples (`core/`)
Basic usage examples and configuration management demonstrations.
- `basic_usage.rs` - Simple introduction to Orasi
- `config_management_example.rs` - Configuration management patterns

### Ingestion Examples (`ingestion/`)
Data ingestion and receiver examples using various protocols.
- `otap_example.rs` - OpenTelemetry Arrow Protocol examples
- `otap_receiver_example.rs` - OTAP receiver implementation
- `minimal_otap_example.rs` - Minimal OTAP setup
- `receiver_example.rs` - General receiver patterns
- `simple_receiver_example.rs` - Basic receiver implementation
- `streaming_exporter_example.rs` - Streaming data export

### Connectors Examples (`connectors/`)
Examples for various data lake and storage connectors.
- `delta_lake_example.rs` - Delta Lake integration
- `real_delta_lake_example.rs` - Production Delta Lake setup
- `hudi_example.rs` - Apache Hudi integration
- `iceberg_example.rs` - Apache Iceberg integration
- `snowflake_example.rs` - Snowflake data warehouse
- `kafka_connector_example.rs` - Kafka connector usage
- `kafka_example.rs` - Kafka integration patterns

### Streaming Examples (`streaming/`)
Stream processing and real-time data pipeline examples.
- `streaming_processor_example.rs` - Stream processing basics
- `streaming_processor_integration.rs` - Integrated streaming workflows
- `batch_compression_example.rs` - Batch compression techniques

### Authentication Examples (`auth/`)
Authentication and authorization examples.
- `auth_example.rs` - Authentication system usage

### Query Engine Examples (`query-engine/`)
Data querying and analysis examples.
- `query_engine_example.rs` - Query engine usage
- `datafusion_query_example.rs` - DataFusion integration

### Bridge API Examples (`bridge-api/`)
API and service integration examples.
- `bridge_api_example.rs` - Bridge API usage
- `http_server_example.rs` - HTTP server implementation
- `schema_registry_shutdown_example.rs` - Schema registry management

## Running Examples

To run a specific example:

```bash
cargo run --example <example_name>
```

For example:
```bash
cargo run --example basic_usage
```

## Prerequisites

Make sure you have the necessary dependencies and services running for the examples you want to try. Some examples require:
- Kafka cluster
- Delta Lake storage
- Snowflake account
- S3 bucket access
- etc.

Check individual example files for specific requirements.
