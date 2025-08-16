# Snowflake Connector

The Snowflake connector for the OpenTelemetry Data Lake Bridge provides seamless integration with Snowflake for storing and querying telemetry data in a lakehouse format.

## Features

- **Full Lakehouse Integration**: Implements the complete `LakehouseConnector` trait for seamless integration with the bridge
- **High-Performance Writing**: Efficient batch writing with configurable batch sizes and flush intervals
- **Advanced Querying**: Support for complex queries with filters, aggregations, and time ranges
- **Comprehensive Configuration**: Extensive configuration options for connection, warehouse, database, and performance settings
- **Health Monitoring**: Built-in health checks and statistics collection
- **Security**: Support for encryption, access control, and role-based permissions
- **Schema Evolution**: Automatic schema evolution and validation
- **Performance Optimization**: Parallel processing, memory optimization, and query acceleration

## Installation

Add the Snowflake connector to your `Cargo.toml`:

```toml
[dependencies]
lakehouse-snowflake = { path = "connectors/snowflake" }
```

## Quick Start

```rust
use lakehouse_snowflake::{SnowflakeConfig, SnowflakeConnector};
use bridge_core::traits::{LakehouseConnector, LakehouseWriter, LakehouseReader};
use bridge_core::types::{MetricsBatch, MetricData, MetricType, MetricValue};
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Snowflake configuration
    let config = SnowflakeConfig {
        connection: lakehouse_snowflake::config::SnowflakeConnectionConfig {
            account: "your-account".to_string(),
            username: "your-username".to_string(),
            password: "your-password".to_string(),
            role: Some("your-role".to_string()),
            warehouse: "COMPUTE_WH".to_string(),
            database: "OPENTELEMETRY".to_string(),
            schema: "PUBLIC".to_string(),
            connection_timeout_secs: 60,
            query_timeout_secs: 300,
        },
        // ... other configuration options
        ..Default::default()
    };

    // Connect to Snowflake
    let connector = SnowflakeConnector::connect(config).await?;
    
    // Get writer and reader handles
    let writer = connector.writer().await?;
    let reader = connector.reader().await?;

    // Write metrics
    let metrics_batch = MetricsBatch {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        metrics: vec![
            MetricData {
                name: "cpu_usage".to_string(),
                description: Some("CPU usage percentage".to_string()),
                unit: Some("percent".to_string()),
                metric_type: MetricType::Gauge,
                value: MetricValue::Gauge(75.5),
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("service".to_string(), "web-server".to_string());
                    labels
                },
                timestamp: Utc::now(),
            },
        ],
        metadata: HashMap::new(),
    };

    let write_result = writer.write_metrics(metrics_batch).await?;
    println!("Wrote {} records", write_result.records_written);

    // Query metrics
    let query = bridge_core::types::MetricsQuery {
        query_id: Uuid::new_v4(),
        timestamp: Utc::now(),
        filters: vec![
            bridge_core::types::Filter {
                field: "service".to_string(),
                operator: bridge_core::types::FilterOperator::Equals,
                value: bridge_core::types::FilterValue::String("web-server".to_string()),
            },
        ],
        aggregations: vec![
            bridge_core::types::Aggregation {
                field: "value".to_string(),
                function: bridge_core::types::AggregationFunction::Average,
                alias: Some("avg_cpu_usage".to_string()),
            },
        ],
        time_range: bridge_core::types::TimeRange {
            start: Utc::now() - chrono::Duration::hours(1),
            end: Utc::now(),
        },
        limit: Some(100),
        offset: Some(0),
        metadata: HashMap::new(),
    };

    let query_result = reader.query_metrics(query).await?;
    println!("Queried {} records", query_result.data.len());

    // Shutdown
    connector.shutdown().await?;
    Ok(())
}
```

## Configuration

The Snowflake connector provides extensive configuration options:

### Connection Configuration

```rust
SnowflakeConnectionConfig {
    account: "your-account".to_string(),
    username: "your-username".to_string(),
    password: "your-password".to_string(),
    role: Some("your-role".to_string()),
    warehouse: "COMPUTE_WH".to_string(),
    database: "OPENTELEMETRY".to_string(),
    schema: "PUBLIC".to_string(),
    connection_timeout_secs: 60,
    query_timeout_secs: 300,
}
```

### Warehouse Configuration

```rust
SnowflakeWarehouseConfig {
    warehouse_name: "COMPUTE_WH".to_string(),
    warehouse_size: "X-SMALL".to_string(),
    auto_suspend_secs: 300,
    auto_resume: true,
    properties: HashMap::new(),
}
```

### Database Configuration

```rust
SnowflakeDatabaseConfig {
    database_name: "OPENTELEMETRY".to_string(),
    schema_name: "PUBLIC".to_string(),
    table_format: "PARQUET".to_string(),
    copy_options: "FILE_FORMAT = (TYPE = 'PARQUET')".to_string(),
}
```

### Writer Configuration

```rust
SnowflakeWriterConfig {
    batch_size: 10000,
    flush_interval_ms: 5000,
    auto_scaling: true,
    enable_clustering: false,
    clustering_keys: Vec::new(),
}
```

### Reader Configuration

```rust
SnowflakeReaderConfig {
    read_batch_size: 10000,
    enable_result_caching: true,
    cache_timeout_secs: 300,
    enable_query_acceleration: false,
    read_timeout_secs: 300,
}
```

### Performance Configuration

```rust
SnowflakePerformanceConfig {
    enable_parallel_processing: true,
    parallel_threads: num_cpus::get(),
    enable_memory_optimization: true,
    memory_limit_mb: 1000,
}
```

### Security Configuration

```rust
SnowflakeSecurityConfig {
    enable_encryption_at_rest: true,
    enable_encryption_in_transit: true,
    enable_access_control: true,
    access_control_mode: Some("rbac".to_string()),
}
```

## API Reference

### SnowflakeConnector

The main connector that implements the `LakehouseConnector` trait.

#### Methods

- `connect(config: SnowflakeConfig) -> BridgeResult<Self>`: Connect to Snowflake with the given configuration
- `writer() -> BridgeResult<SnowflakeWriter>`: Get a writer handle for writing data
- `reader() -> BridgeResult<SnowflakeReader>`: Get a reader handle for reading data
- `health_check() -> BridgeResult<bool>`: Check if the connector is healthy
- `get_stats() -> BridgeResult<ConnectorStats>`: Get connector statistics
- `shutdown() -> BridgeResult<()>`: Shutdown the connector gracefully

### SnowflakeWriter

Implements the `LakehouseWriter` trait for writing telemetry data to Snowflake.

#### Methods

- `write_metrics(batch: MetricsBatch) -> BridgeResult<WriteResult>`: Write metrics batch
- `write_traces(batch: TracesBatch) -> BridgeResult<WriteResult>`: Write traces batch
- `write_logs(batch: LogsBatch) -> BridgeResult<WriteResult>`: Write logs batch
- `write_batch(batch: TelemetryBatch) -> BridgeResult<WriteResult>`: Write any telemetry batch
- `flush() -> BridgeResult<()>`: Flush pending writes
- `get_stats() -> BridgeResult<WriterStats>`: Get writer statistics
- `close() -> BridgeResult<()>`: Close the writer

### SnowflakeReader

Implements the `LakehouseReader` trait for reading telemetry data from Snowflake.

#### Methods

- `query_metrics(query: MetricsQuery) -> BridgeResult<MetricsResult>`: Query metrics
- `query_traces(query: TracesQuery) -> BridgeResult<TracesResult>`: Query traces
- `query_logs(query: LogsQuery) -> BridgeResult<LogsResult>`: Query logs
- `execute_query(query: String) -> BridgeResult<serde_json::Value>`: Execute custom SQL query
- `get_stats() -> BridgeResult<ReaderStats>`: Get reader statistics
- `close() -> BridgeResult<()>`: Close the reader

## Examples

See the `examples/connectors/snowflake_example.rs` file for a complete working example.

## Error Handling

The connector provides comprehensive error handling with detailed error messages and context. All errors are wrapped in `SnowflakeResult<T>` which provides:

- Connection errors
- Authentication errors
- Query execution errors
- Data validation errors
- Configuration errors

## Performance Considerations

- **Batch Size**: Configure appropriate batch sizes for your data volume and latency requirements
- **Warehouse Size**: Choose warehouse size based on your query complexity and performance needs
- **Auto-suspend**: Enable auto-suspend to save costs when not actively querying
- **Clustering**: Use clustering keys for frequently queried columns to improve query performance
- **Parallel Processing**: Enable parallel processing for large datasets
- **Query Acceleration**: Enable query acceleration for complex analytical queries

## Security

The connector supports various security features:

- **Encryption at Rest**: Data is encrypted when stored in Snowflake
- **Encryption in Transit**: All communication is encrypted using TLS
- **Access Control**: Role-based access control (RBAC) support
- **Authentication**: Username/password authentication with optional role specification

## Monitoring

The connector provides comprehensive monitoring capabilities:

- **Health Checks**: Regular health checks to ensure connectivity
- **Statistics**: Detailed statistics on reads, writes, errors, and performance
- **Metrics**: Performance metrics including average response times
- **Error Tracking**: Detailed error tracking with context

## Troubleshooting

### Common Issues

1. **Connection Timeout**: Increase `connection_timeout_secs` in the configuration
2. **Query Timeout**: Increase `query_timeout_secs` for complex queries
3. **Authentication Errors**: Verify username, password, and role settings
4. **Permission Errors**: Ensure the user has appropriate permissions on the database and schema

### Debugging

Enable debug logging to get detailed information:

```rust
tracing_subscriber::fmt()
    .with_env_filter("lakehouse_snowflake=debug")
    .init();
```

## Contributing

Contributions are welcome! Please see the main project contributing guidelines.

## License

This project is licensed under the Apache License 2.0 - see the LICENSE file for details.
