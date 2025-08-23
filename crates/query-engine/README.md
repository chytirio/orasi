# Query Engine

High-performance SQL query engine for the Orasi OpenTelemetry Data Lake Bridge, providing efficient data analysis and querying capabilities for telemetry data.

## Overview

The Query Engine provides a powerful SQL interface for analyzing OpenTelemetry telemetry data stored in the bridge, featuring:

- **SQL Query Processing**: Full SQL support with DataFusion integration
- **Telemetry Data Analysis**: Specialized functions for OpenTelemetry data
- **High Performance**: Optimized query execution with parallel processing
- **Multiple Data Sources**: Support for various storage backends
- **Real-time Queries**: Streaming query capabilities for live data
- **Query Optimization**: Automatic query optimization and caching

## Key Features

### SQL Query Processing
- **Full SQL Support**: Standard SQL with OpenTelemetry extensions
- **DataFusion Integration**: Apache Arrow-based query engine
- **Advanced Query Optimization**: Cost-based optimization with predicate pushdown, join reordering, and more
- **Parallel Execution**: Multi-threaded query processing
- **Memory Management**: Efficient memory usage and garbage collection
- **Query Caching**: Intelligent result caching with TTL and LRU eviction

### OpenTelemetry Integration
- **Telemetry Functions**: Specialized functions for telemetry data analysis including error rates, percentiles, and trace analysis
- **Schema Awareness**: Automatic schema detection and validation
- **Signal Support**: Traces, metrics, and logs querying
- **Semantic Conventions**: Built-in support for OpenTelemetry semantic conventions with automatic attribute extraction
- **Time Series Analysis**: Time-based querying and aggregation with moving averages and rate of change calculations
- **Service Dependency Analysis**: Built-in functions for analyzing service relationships and dependencies

### Data Source Support
- **Parquet Files**: Direct querying of Parquet data files
- **Delta Lake**: Delta Lake table format support
- **Iceberg**: Apache Iceberg table format support
- **Streaming Sources**: Real-time data streaming and querying
- **Custom Connectors**: Extensible connector framework

### Performance Features
- **Query Caching**: Intelligent query result caching with TTL and LRU eviction
- **Query Optimization**: Cost-based optimization with multiple optimization strategies
- **Indexing**: Automatic index creation and usage
- **Partitioning**: Partition-aware query optimization
- **Compression**: Data compression and decompression
- **Vectorization**: SIMD-optimized data processing
- **Performance Monitoring**: Comprehensive statistics and metrics collection

## Quick Start

### Basic Usage

```rust
use query_engine::{
    QueryEngine, QueryEngineConfig,
    executors::DataFusionExecutor,
    parsers::SqlParser,
    optimizers::QueryOptimizer,
};

#[tokio::main]
async fn main() -> query_engine::Result<()> {
    // Initialize query engine
    let config = QueryEngineConfig::new()
        .with_max_memory_mb(1024)
        .with_parallelism(4);
    
    let query_engine = QueryEngine::new(config);
    
    // Register data sources
    query_engine.register_data_source(
        "telemetry_data",
        "parquet:///path/to/telemetry/data"
    ).await?;
    
    // Execute a simple query
    let sql = "
        SELECT 
            service.name,
            COUNT(*) as request_count,
            AVG(duration_ms) as avg_duration
        FROM telemetry_data 
        WHERE timestamp >= NOW() - INTERVAL '1 hour'
        GROUP BY service.name
        ORDER BY request_count DESC
        LIMIT 10
    ";
    
    let result = query_engine.execute_query(sql).await?;
    
    // Process results
    for row in result.rows {
        println!(
            "Service: {}, Requests: {}, Avg Duration: {}ms",
            row.get("service.name")?,
            row.get("request_count")?,
            row.get("avg_duration")?
        );
    }
    
    Ok(())
}
```

### Advanced Querying

```rust
use query_engine::{
    QueryEngine, TelemetryFunctions,
    executors::StreamingExecutor,
};

#[tokio::main]
async fn main() -> query_engine::Result<()> {
    let query_engine = QueryEngine::new(QueryEngineConfig::default());
    
    // Register telemetry functions
    let telemetry_functions = TelemetryFunctions::new();
    query_engine.register_functions(telemetry_functions);
    
    // Complex telemetry analysis query
    let sql = "
        WITH error_rates AS (
            SELECT 
                service.name,
                DATE_TRUNC('hour', timestamp) as hour,
                COUNT(CASE WHEN status_code >= 400 THEN 1 END) as error_count,
                COUNT(*) as total_requests,
                COUNT(CASE WHEN status_code >= 400 THEN 1 END) * 100.0 / COUNT(*) as error_rate
            FROM telemetry_data
            WHERE timestamp >= NOW() - INTERVAL '24 hours'
            GROUP BY service.name, DATE_TRUNC('hour', timestamp)
        )
        SELECT 
            service_name,
            hour,
            error_rate,
            LAG(error_rate) OVER (PARTITION BY service_name ORDER BY hour) as prev_error_rate,
            error_rate - LAG(error_rate) OVER (PARTITION BY service_name ORDER BY hour) as error_rate_change
        FROM error_rates
        WHERE error_rate > 5.0
        ORDER BY error_rate_change DESC
    ";
    
    let result = query_engine.execute_query(sql).await?;
    
    // Process streaming results
    let mut stream = query_engine.execute_streaming_query(sql).await?;
    
    while let Some(batch) = stream.next().await? {
        for row in batch.rows {
            println!(
                "Service: {}, Hour: {}, Error Rate: {:.2}%, Change: {:.2}%",
                row.get("service_name")?,
                row.get("hour")?,
                row.get("error_rate")?,
                row.get("error_rate_change")?
            );
        }
    }
    
    Ok(())
}
```

### Configuration Example

```toml
# query-engine.toml
[query_engine]
max_memory_mb = 2048
parallelism = 8
cache_size_mb = 512
query_timeout_seconds = 300

[query_engine.data_sources]
telemetry_data = "parquet:///data/telemetry"
metrics_data = "delta:///data/metrics"
logs_data = "iceberg:///data/logs"

[query_engine.optimization]
enable_parallel_execution = true
enable_query_cache = true
enable_index_usage = true
max_partition_size_mb = 100

[query_engine.telemetry]
enable_semantic_conventions = true
enable_telemetry_functions = true
default_timezone = "UTC"

[query_engine.monitoring]
enable_query_metrics = true
enable_slow_query_logging = true
slow_query_threshold_ms = 1000
```

## SQL Extensions

### Telemetry Functions

```sql
-- Get service dependencies
SELECT * FROM get_service_dependencies('my-service', '1 hour');

-- Calculate error rate
SELECT service.name, calculate_error_rate(status_code) as error_rate
FROM telemetry_data
WHERE timestamp >= NOW() - INTERVAL '1 hour';

-- Get latency percentiles
SELECT 
    service.name,
    percentile_cont(duration_ms, 0.95) as p95_latency,
    percentile_cont(duration_ms, 0.99) as p99_latency
FROM telemetry_data
GROUP BY service.name;

-- Analyze trace spans
SELECT 
    span_name,
    COUNT(*) as span_count,
    AVG(duration_ms) as avg_duration
FROM get_trace_spans('trace_id_123')
GROUP BY span_name;
```

### Time Series Functions

```sql
-- Time-based aggregations
SELECT 
    DATE_TRUNC('minute', timestamp) as minute,
    COUNT(*) as request_count,
    AVG(duration_ms) as avg_duration
FROM telemetry_data
WHERE timestamp >= NOW() - INTERVAL '1 hour'
GROUP BY DATE_TRUNC('minute', timestamp)
ORDER BY minute;

-- Moving averages
SELECT 
    timestamp,
    request_count,
    AVG(request_count) OVER (
        ORDER BY timestamp 
        ROWS BETWEEN 5 PRECEDING AND CURRENT ROW
    ) as moving_avg_5min
FROM (
    SELECT 
        DATE_TRUNC('minute', timestamp) as timestamp,
        COUNT(*) as request_count
    FROM telemetry_data
    WHERE timestamp >= NOW() - INTERVAL '1 hour'
    GROUP BY DATE_TRUNC('minute', timestamp)
);
```

## Architecture

The Query Engine follows a layered architecture with clear separation of concerns:

```
┌─────────────────┐
│   Query Engine  │
├─────────────────┤
│  SQL Parser     │
│  Query Optimizer│
│  Executors      │
│  Data Sources   │
│  Functions      │
│  Cache          │
└─────────────────┘
```

### Core Components

1. **SQL Parser**: SQL query parsing and validation
2. **Query Optimizer**: Query plan optimization and execution planning
3. **Executors**: Query execution engines (DataFusion, streaming)
4. **Data Sources**: Connectors to various data storage systems
5. **Functions**: Built-in and custom SQL functions
6. **Cache**: Query result and plan caching

## Performance Optimization

### Query Optimization
- **Cost-based optimization**: Automatic query plan optimization
- **Predicate pushdown**: Early filtering at data source level
- **Column pruning**: Only read required columns
- **Join optimization**: Efficient join strategies
- **Aggregation optimization**: Optimized aggregation algorithms

### Memory Management
- **Memory pools**: Efficient memory allocation and reuse
- **Garbage collection**: Automatic memory cleanup
- **Spill to disk**: Handle large datasets that don't fit in memory
- **Compression**: Data compression to reduce memory usage

### Parallel Processing
- **Multi-threading**: Parallel query execution
- **Partitioning**: Data partitioning for parallel processing
- **Pipeline parallelism**: Overlapping I/O and computation
- **Load balancing**: Dynamic work distribution

## Dependencies

### Core Dependencies
- **tokio**: Async runtime
- **serde**: Serialization/deserialization
- **tracing**: Structured logging
- **anyhow**: Error handling
- **thiserror**: Error type generation

### Data Processing
- **datafusion**: Apache Arrow-based query engine
- **arrow**: Apache Arrow data format
- **parquet**: Parquet file format support
- **deltalake**: Delta Lake table format
- **iceberg-rust**: Apache Iceberg support

### OpenTelemetry
- **opentelemetry**: Core OpenTelemetry functionality
- **opentelemetry-semantic-conventions**: Semantic conventions

### Utilities
- **async-trait**: Async trait support
- **dashmap**: Concurrent hash map
- **crossbeam-channel**: Cross-thread communication
- **parking_lot**: Efficient synchronization primitives

## Development

### Building

```bash
# Build the crate
cargo build

# Build with optimizations
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_sql_parser

# Run integration tests
cargo test --test integration

# Run with logging
RUST_LOG=debug cargo test
```

### Benchmarking

```bash
# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench query_execution

# Run with custom parameters
cargo bench -- --benchmark-iterations 100
```

### Documentation

```bash
# Generate documentation
cargo doc

# Open documentation in browser
cargo doc --open
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.
