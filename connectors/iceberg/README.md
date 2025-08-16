# Apache Iceberg Connector

A high-performance Apache Iceberg connector for the OpenTelemetry Data Lake Bridge, providing seamless integration with Iceberg tables for storing and querying telemetry data.

## Features

- **Full Iceberg Support**: Complete integration with Apache Iceberg table format
- **Telemetry Data Types**: Support for metrics, traces, and logs
- **Schema Evolution**: Automatic schema management and evolution
- **Partitioning**: Configurable partitioning strategies for optimal performance
- **Compression**: Multiple compression codec support (Snappy, GZIP, etc.)
- **Catalog Integration**: Support for Hive, Nessie, and custom catalogs
- **Batch Processing**: Efficient batch writing and reading operations
- **Monitoring**: Built-in metrics and health checks
- **Error Handling**: Comprehensive error handling and recovery

## Quick Start

### Installation

Add the Iceberg connector to your `Cargo.toml`:

```toml
[dependencies]
lakehouse-iceberg = { path = "connectors/iceberg" }
```

### Basic Usage

```rust
use lakehouse_iceberg::{IcebergConnector, IcebergConfig};
use bridge_core::traits::LakehouseConnector;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration
    let config = IcebergConfig::new(
        "s3://my-iceberg-warehouse".to_string(),
        "telemetry_data".to_string(),
    );

    // Connect to Iceberg
    let connector = IcebergConnector::connect(config).await?;

    // Get writer and reader
    let writer = connector.writer().await?;
    let reader = connector.reader().await?;

    // Use writer and reader for data operations
    // ...

    Ok(())
}
```

## Configuration

### Basic Configuration

```rust
use lakehouse_iceberg::config::IcebergConfig;

let config = IcebergConfig {
    storage: IcebergStorageConfig {
        storage_path: "s3://my-warehouse".to_string(),
        storage_type: "s3".to_string(),
        options: HashMap::new(),
    },
    table: IcebergTableConfig {
        table_name: "telemetry_data".to_string(),
        table_format_version: 2,
        partition_columns: vec!["year".to_string(), "month".to_string(), "day".to_string()],
        compression: "snappy".to_string(),
        properties: HashMap::new(),
    },
    catalog: IcebergCatalogConfig {
        catalog_type: "hive".to_string(),
        catalog_uri: Some("thrift://localhost:9083".to_string()),
        warehouse_location: "s3://my-warehouse".to_string(),
        properties: HashMap::new(),
    },
    ..Default::default()
};
```

### Configuration Options

#### Storage Configuration
- `storage_path`: Base path for Iceberg tables
- `storage_type`: Storage backend (s3, local, azure, gcs)
- `options`: Storage-specific configuration options

#### Table Configuration
- `table_name`: Name of the Iceberg table
- `table_format_version`: Iceberg table format version (1 or 2)
- `partition_columns`: List of columns to partition by
- `compression`: Compression codec (snappy, gzip, zstd)
- `properties`: Table-specific properties

#### Writer Configuration
- `batch_size`: Number of records per batch
- `flush_interval_ms`: Flush interval in milliseconds
- `auto_compact`: Enable automatic compaction
- `compact_interval_hours`: Compaction interval

#### Reader Configuration
- `read_batch_size`: Batch size for reading
- `enable_predicate_pushdown`: Enable predicate pushdown
- `enable_column_pruning`: Enable column pruning
- `enable_partition_pruning`: Enable partition pruning
- `read_timeout_secs`: Read timeout in seconds

## Data Types

### Metrics
```rust
use bridge_core::types::{MetricData, MetricType, MetricValue};

let metric = MetricData {
    name: "cpu_usage".to_string(),
    description: Some("CPU usage percentage".to_string()),
    unit: Some("percent".to_string()),
    metric_type: MetricType::Gauge,
    value: MetricValue::Gauge(75.5),
    labels: HashMap::new(),
    timestamp: chrono::Utc::now(),
};
```

### Traces
```rust
use bridge_core::types::TraceData;

let trace = TraceData {
    trace_id: "1234567890abcdef".to_string(),
    span_id: "abcdef1234567890".to_string(),
    parent_span_id: None,
    operation_name: "http_request".to_string(),
    span_kind: "server".to_string(),
    span_status: "ok".to_string(),
    span_duration_ms: 150.0,
    attributes: HashMap::new(),
    timestamp: chrono::Utc::now(),
};
```

### Logs
```rust
use bridge_core::types::LogData;

let log = LogData {
    level: "info".to_string(),
    message: "Request processed successfully".to_string(),
    attributes: HashMap::new(),
    timestamp: chrono::Utc::now(),
};
```

## Writing Data

### Using the Writer

```rust
use bridge_core::types::{MetricsBatch, TracesBatch, LogsBatch};

// Write metrics
let metrics_batch = MetricsBatch {
    id: uuid::Uuid::new_v4(),
    timestamp: chrono::Utc::now(),
    metrics: vec![metric],
    metadata: HashMap::new(),
};
writer.write_metrics(metrics_batch).await?;

// Write traces
let traces_batch = TracesBatch {
    id: uuid::Uuid::new_v4(),
    timestamp: chrono::Utc::now(),
    traces: vec![trace],
    metadata: HashMap::new(),
};
writer.write_traces(traces_batch).await?;

// Write logs
let logs_batch = LogsBatch {
    id: uuid::Uuid::new_v4(),
    timestamp: chrono::Utc::now(),
    logs: vec![log],
    metadata: HashMap::new(),
};
writer.write_logs(logs_batch).await?;
```

## Reading Data

### Using the Reader

```rust
use bridge_core::types::{MetricsQuery, TracesQuery, LogsQuery};

// Query metrics
let metrics_query = MetricsQuery {
    time_range: TimeRange {
        start: chrono::Utc::now() - chrono::Duration::hours(1),
        end: chrono::Utc::now(),
    },
    filters: vec!["service_name = 'web-server'".to_string()],
    limit: Some(1000),
};

let metrics_result = reader.query_metrics(metrics_query).await?;
println!("Found {} metrics", metrics_result.total_count);

// Query traces
let traces_query = TracesQuery {
    time_range: TimeRange {
        start: chrono::Utc::now() - chrono::Duration::hours(1),
        end: chrono::Utc::now(),
    },
    filters: vec!["operation_name = 'http_request'".to_string()],
    limit: Some(1000),
};

let traces_result = reader.query_traces(traces_query).await?;
println!("Found {} traces", traces_result.total_count);

// Query logs
let logs_query = LogsQuery {
    time_range: TimeRange {
        start: chrono::Utc::now() - chrono::Duration::hours(1),
        end: chrono::Utc::now(),
    },
    filters: vec!["level = 'error'".to_string()],
    limit: Some(1000),
};

let logs_result = reader.query_logs(logs_query).await?;
println!("Found {} logs", logs_result.total_count);
```

## Schema Management

The connector automatically manages Iceberg schemas and supports schema evolution:

```rust
use lakehouse_iceberg::schema::IcebergSchema;

let schema = IcebergSchema::new(config.schema.clone()).await?;

// Get current schema
let current_schema = schema.get_schema().await?;

// Check schema version
let version = schema.version();
println!("Current schema version: {}", version);
```

## Monitoring and Health Checks

### Health Checks

```rust
// Check connector health
let is_healthy = connector.health_check().await?;
if is_healthy {
    println!("Connector is healthy");
} else {
    println!("Connector health check failed");
}
```

### Statistics

```rust
// Get connector statistics
let stats = connector.get_stats().await?;
println!("Total connections: {}", stats.total_connections);
println!("Total writes: {}", stats.total_writes);
println!("Total reads: {}", stats.total_reads);
println!("Error count: {}", stats.error_count);
```

## Error Handling

The connector provides comprehensive error handling:

```rust
use lakehouse_iceberg::error::{IcebergError, IcebergResult};

match connector.connect(config).await {
    Ok(connector) => {
        println!("Successfully connected to Iceberg");
    }
    Err(IcebergError::Configuration(e)) => {
        eprintln!("Configuration error: {}", e);
    }
    Err(IcebergError::Connection(e)) => {
        eprintln!("Connection error: {}", e);
    }
    Err(IcebergError::Validation(e)) => {
        eprintln!("Validation error: {}", e);
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

## Examples

See the `examples/` directory for complete working examples:

- `iceberg_example.rs`: Basic usage example
- Integration examples with the bridge pipeline

## Testing

Run the test suite:

```bash
cargo test
```

Run specific tests:

```bash
cargo test test_iceberg_connector_initialization
```

## Performance Considerations

1. **Batch Size**: Adjust batch size based on your data volume and memory constraints
2. **Partitioning**: Choose appropriate partition columns for your query patterns
3. **Compression**: Use Snappy for good compression/speed balance
4. **Parallel Processing**: Enable parallel processing for large datasets
5. **Memory Optimization**: Configure memory limits appropriately

## Troubleshooting

### Common Issues

1. **Connection Errors**: Verify catalog URI and warehouse location
2. **Permission Errors**: Check AWS credentials and S3 permissions
3. **Schema Errors**: Ensure table schema matches your data
4. **Performance Issues**: Adjust batch sizes and partitioning

### Debug Logging

Enable debug logging:

```rust
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_env_filter("lakehouse_iceberg=debug")
    .init();
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

Apache License 2.0 - see LICENSE file for details.

## Support

For issues and questions:
- GitHub Issues: [Create an issue](https://github.com/chytirio/orasi/issues)
- Documentation: [Project docs](https://github.com/chytirio/orasi/docs)
