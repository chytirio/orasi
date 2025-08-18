# S3 Parquet Connector

Amazon S3 and Apache Parquet connector for the Orasi OpenTelemetry Data Lake Bridge, providing efficient storage and retrieval of telemetry data in S3 using the Parquet format.

## Overview

The S3 Parquet connector enables the Orasi bridge to store and retrieve OpenTelemetry telemetry data in Amazon S3 using the Apache Parquet columnar format, offering:

- **S3 Integration**: Native Amazon S3 storage and retrieval
- **Parquet Format**: High-performance columnar storage format
- **Partitioning**: Intelligent data partitioning strategies
- **Compression**: Multiple compression algorithms support
- **Batch Operations**: Efficient batch read and write operations
- **Streaming**: Streaming data ingestion and retrieval
- **Cost Optimization**: S3 cost optimization features

## Key Features

### S3 Storage
- **Multi-region Support**: Support for multiple AWS regions
- **Bucket Management**: Automatic bucket creation and management
- **Object Lifecycle**: Configurable object lifecycle policies
- **Versioning**: S3 object versioning support
- **Encryption**: Server-side and client-side encryption
- **Access Control**: Fine-grained access control with IAM

### Parquet Format
- **Columnar Storage**: Efficient columnar data storage
- **Schema Evolution**: Automatic schema evolution handling
- **Compression**: Multiple compression algorithms (SNAPPY, GZIP, LZ4)
- **Predicate Pushdown**: Efficient filtering and query optimization
- **Statistics**: Built-in column statistics for optimization
- **Type Support**: Full support for all Parquet data types

### Data Operations
- **Batch Writes**: Efficient batch write operations
- **Streaming Writes**: Real-time streaming data ingestion
- **Partitioned Reads**: Optimized partitioned data reading
- **Incremental Processing**: Incremental data processing support
- **Data Validation**: Data quality validation and enforcement
- **Error Handling**: Comprehensive error handling and recovery

### Performance Optimization
- **Parallel Processing**: Multi-threaded read and write operations
- **Memory Management**: Efficient memory usage and garbage collection
- **Caching**: Intelligent caching strategies
- **Compression**: Configurable compression levels
- **Partitioning**: Optimal partitioning strategies

## Quick Start

### Basic Usage

```rust
use s3_parquet_connector::{
    S3ParquetConnector, S3Config, ParquetConfig,
    operations::{WriteOperation, ReadOperation},
    schema::ParquetSchema,
};

#[tokio::main]
async fn main() -> s3_parquet_connector::Result<()> {
    // Configure S3 connector
    let s3_config = S3Config::new()
        .with_region("us-east-1")
        .with_bucket("my-telemetry-bucket")
        .with_prefix("telemetry-data")
        .with_credentials_from_env();
    
    let parquet_config = ParquetConfig::new()
        .with_compression("SNAPPY")
        .with_row_group_size(100000)
        .with_page_size(1024 * 1024);
    
    let connector = S3ParquetConnector::new(s3_config, parquet_config);
    
    // Define schema
    let schema = ParquetSchema::new()
        .with_field("id", "string", true)
        .with_field("timestamp", "timestamp", false)
        .with_field("service_name", "string", false)
        .with_field("duration_ms", "int64", false)
        .with_field("status_code", "int32", false);
    
    // Write data
    let write_op = WriteOperation::new()
        .with_schema(schema)
        .with_records(vec![
            serde_json::json!({
                "id": "span_123",
                "timestamp": "2024-01-01T00:00:00Z",
                "service_name": "web-service",
                "duration_ms": 150,
                "status_code": 200
            })
        ])
        .with_partition_path("service_name=web-service/date=2024-01-01");
    
    let result = connector.write(write_op).await?;
    println!("Written {} records to {}", result.record_count, result.path);
    
    // Read data
    let read_op = ReadOperation::new()
        .with_partition_path("service_name=web-service/date=2024-01-01")
        .with_filter("status_code = 200");
    
    let data = connector.read(read_op).await?;
    for record in data.records {
        println!("Record: {:?}", record);
    }
    
    Ok(())
}
```

### Advanced Configuration

```rust
use s3_parquet_connector::{
    S3ParquetConnector, S3Config, ParquetConfig,
    operations::{BatchWriteOperation, StreamingReadOperation},
    partitioning::TimePartitioning,
};

#[tokio::main]
async fn main() -> s3_parquet_connector::Result<()> {
    // Advanced S3 configuration
    let s3_config = S3Config::new()
        .with_region("us-east-1")
        .with_bucket("my-telemetry-bucket")
        .with_prefix("telemetry-data")
        .with_credentials_from_env()
        .with_encryption("AES256")
        .with_multipart_threshold(100 * 1024 * 1024)  // 100MB
        .with_max_concurrent_uploads(10)
        .with_retry_attempts(3);
    
    // Advanced Parquet configuration
    let parquet_config = ParquetConfig::new()
        .with_compression("SNAPPY")
        .with_row_group_size(100000)
        .with_page_size(1024 * 1024)
        .with_dictionary_enabled(true)
        .with_statistics_enabled(true)
        .with_bloom_filter_enabled(true);
    
    let connector = S3ParquetConnector::new(s3_config, parquet_config);
    
    // Configure time-based partitioning
    let partitioning = TimePartitioning::new()
        .with_field("timestamp")
        .with_granularity("hour")
        .with_format("%Y-%m-%d/%H");
    
    // Batch write operation
    let batch_op = BatchWriteOperation::new()
        .with_records(generate_telemetry_data())
        .with_partitioning(partitioning)
        .with_parallelism(4);
    
    let result = connector.batch_write(batch_op).await?;
    println!("Batch written {} files", result.file_count);
    
    // Streaming read operation
    let stream_op = StreamingReadOperation::new()
        .with_partition_path("service_name=web-service")
        .with_filter("timestamp >= '2024-01-01'")
        .with_batch_size(1000);
    
    let mut stream = connector.streaming_read(stream_op).await?;
    while let Some(batch) = stream.next().await? {
        println!("Received batch with {} records", batch.records.len());
    }
    
    Ok(())
}

fn generate_telemetry_data() -> Vec<serde_json::Value> {
    (0..10000).map(|i| {
        serde_json::json!({
            "id": format!("span_{}", i),
            "timestamp": "2024-01-01T00:00:00Z",
            "service_name": "web-service",
            "duration_ms": 100 + (i % 200),
            "status_code": if i % 10 == 0 { 500 } else { 200 }
        })
    }).collect()
}
```

### Configuration Example

```toml
# s3-parquet.toml
[s3]
region = "us-east-1"
bucket = "my-telemetry-bucket"
prefix = "telemetry-data"
endpoint = null  # Use default AWS endpoint

[s3.credentials]
type = "env"  # env, profile, explicit
profile = "default"
access_key_id = null
secret_access_key = null

[s3.encryption]
type = "AES256"  # AES256, aws:kms
kms_key_id = null

[s3.multipart]
enabled = true
threshold_mb = 100
max_concurrent_uploads = 10

[parquet]
compression = "SNAPPY"  # SNAPPY, GZIP, LZ4, ZSTD
row_group_size = 100000
page_size = 1048576  # 1MB
dictionary_enabled = true
statistics_enabled = true
bloom_filter_enabled = true

[partitioning]
type = "time"  # time, hash, range
field = "timestamp"
granularity = "hour"
format = "%Y-%m-%d/%H"

[operations]
batch_size = 10000
parallelism = 4
retry_attempts = 3
timeout_seconds = 300
```

## API Reference

### S3ParquetConnector

```rust
pub struct S3ParquetConnector {
    s3_config: S3Config,
    parquet_config: ParquetConfig,
}

impl S3ParquetConnector {
    // Create new connector
    pub fn new(s3_config: S3Config, parquet_config: ParquetConfig) -> Self;
    
    // Write operation
    pub async fn write(&self, operation: WriteOperation) -> Result<WriteResult>;
    
    // Read operation
    pub async fn read(&self, operation: ReadOperation) -> Result<ReadResult>;
    
    // Batch write operation
    pub async fn batch_write(&self, operation: BatchWriteOperation) -> Result<BatchWriteResult>;
    
    // Streaming read operation
    pub async fn streaming_read(&self, operation: StreamingReadOperation) -> Result<ReadStream>;
    
    // List partitions
    pub async fn list_partitions(&self, prefix: &str) -> Result<Vec<String>>;
    
    // Delete partition
    pub async fn delete_partition(&self, partition_path: &str) -> Result<()>;
}
```

### Operations

#### WriteOperation

```rust
pub struct WriteOperation {
    schema: ParquetSchema,
    records: Vec<serde_json::Value>,
    partition_path: Option<String>,
    file_name: Option<String>,
}

impl WriteOperation {
    pub fn new() -> Self;
    pub fn with_schema(mut self, schema: ParquetSchema) -> Self;
    pub fn with_records(mut self, records: Vec<serde_json::Value>) -> Self;
    pub fn with_partition_path(mut self, path: String) -> Self;
    pub fn with_file_name(mut self, name: String) -> Self;
}
```

#### ReadOperation

```rust
pub struct ReadOperation {
    partition_path: Option<String>,
    filter: Option<String>,
    select_fields: Vec<String>,
    limit: Option<usize>,
}

impl ReadOperation {
    pub fn new() -> Self;
    pub fn with_partition_path(mut self, path: String) -> Self;
    pub fn with_filter(mut self, filter: String) -> Self;
    pub fn with_select_fields(mut self, fields: Vec<String>) -> Self;
    pub fn with_limit(mut self, limit: usize) -> Self;
}
```

#### BatchWriteOperation

```rust
pub struct BatchWriteOperation {
    records: Vec<serde_json::Value>,
    partitioning: Option<Box<dyn Partitioning>>,
    parallelism: usize,
    batch_size: usize,
}

impl BatchWriteOperation {
    pub fn new() -> Self;
    pub fn with_records(mut self, records: Vec<serde_json::Value>) -> Self;
    pub fn with_partitioning(mut self, partitioning: Box<dyn Partitioning>) -> Self;
    pub fn with_parallelism(mut self, parallelism: usize) -> Self;
    pub fn with_batch_size(mut self, batch_size: usize) -> Self;
}
```

## Schema Management

### Schema Definition

```rust
use s3_parquet_connector::schema::{ParquetSchema, ParquetField, ParquetType};

let schema = ParquetSchema::new()
    .with_field("id", ParquetType::String, true)
    .with_field("timestamp", ParquetType::Timestamp, false)
    .with_field("service_name", ParquetType::String, false)
    .with_field("duration_ms", ParquetType::Int64, false)
    .with_field("status_code", ParquetType::Int32, false)
    .with_field("attributes", ParquetType::Map, false);
```

### Schema Evolution

```rust
// Add new field
let evolved_schema = schema
    .with_field("user_id", ParquetType::String, false)
    .with_field("session_id", ParquetType::String, false);

// Validate schema compatibility
connector.validate_schema_compatibility(&old_schema, &evolved_schema).await?;
```

## Partitioning Strategies

### Time-based Partitioning

```rust
use s3_parquet_connector::partitioning::TimePartitioning;

let partitioning = TimePartitioning::new()
    .with_field("timestamp")
    .with_granularity("hour")
    .with_format("%Y-%m-%d/%H");

// This will create partitions like:
// service_name=web-service/2024-01-01/00/
// service_name=web-service/2024-01-01/01/
```

### Hash-based Partitioning

```rust
use s3_parquet_connector::partitioning::HashPartitioning;

let partitioning = HashPartitioning::new()
    .with_field("service_name")
    .with_num_partitions(10);
```

### Range-based Partitioning

```rust
use s3_parquet_connector::partitioning::RangePartitioning;

let partitioning = RangePartitioning::new()
    .with_field("status_code")
    .with_ranges(vec![
        (0, 200, "success"),
        (200, 400, "client_error"),
        (400, 600, "server_error"),
    ]);
```

## Performance Optimization

### Compression Strategies

```rust
// Optimize for read performance
let config = ParquetConfig::new()
    .with_compression("SNAPPY")
    .with_row_group_size(100000)
    .with_page_size(1024 * 1024);

// Optimize for storage size
let config = ParquetConfig::new()
    .with_compression("GZIP")
    .with_row_group_size(50000)
    .with_page_size(512 * 1024);
```

### Parallel Processing

```rust
// Configure parallel processing
let batch_op = BatchWriteOperation::new()
    .with_records(records)
    .with_parallelism(8)
    .with_batch_size(5000);
```

### S3 Optimization

```rust
// Optimize S3 operations
let s3_config = S3Config::new()
    .with_multipart_threshold(100 * 1024 * 1024)  // 100MB
    .with_max_concurrent_uploads(10)
    .with_retry_attempts(3)
    .with_timeout_seconds(300);
```

## Error Handling

```rust
use s3_parquet_connector::error::S3ParquetError;

match connector.write(operation).await {
    Ok(result) => {
        println!("Write successful: {} records", result.record_count);
    }
    Err(S3ParquetError::S3Error { source }) => {
        eprintln!("S3 error: {}", source);
    }
    Err(S3ParquetError::ParquetError { source }) => {
        eprintln!("Parquet error: {}", source);
    }
    Err(S3ParquetError::SchemaError { message }) => {
        eprintln!("Schema error: {}", message);
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

## Monitoring and Metrics

### Operation Metrics

```rust
// Get write metrics
let metrics = connector.get_write_metrics().await?;
println!("Write latency: {:?}", metrics.avg_latency);
println!("Write throughput: {} records/sec", metrics.throughput);
println!("Compression ratio: {:.2}", metrics.compression_ratio);

// Get read metrics
let metrics = connector.get_read_metrics().await?;
println!("Read latency: {:?}", metrics.avg_latency);
println!("Read throughput: {} records/sec", metrics.throughput);
```

### S3 Metrics

```rust
// Get S3 operation metrics
let s3_metrics = connector.get_s3_metrics().await?;
println!("S3 requests: {}", s3_metrics.request_count);
println!("S3 errors: {}", s3_metrics.error_count);
println!("S3 latency: {:?}", s3_metrics.avg_latency);
```

## Development

### Building

```bash
# Build connector
cargo build

# Build with optimizations
cargo build --release

# Build with features
cargo build --features s3-native
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_write_operation

# Run integration tests
cargo test --test integration

# Run with logging
RUST_LOG=debug cargo test
```

### Examples

```bash
# Run examples
cargo run --example basic_usage
cargo run --example batch_operations
cargo run --example partitioning
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
