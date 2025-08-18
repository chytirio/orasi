# Apache Hudi Connector

Apache Hudi connector for the Orasi OpenTelemetry Data Lake Bridge, providing seamless integration with Hudi data lakes for telemetry data storage and management.

## Overview

The Apache Hudi connector enables the Orasi bridge to store and manage OpenTelemetry telemetry data in Hudi data lakes, offering:

- **Hudi Integration**: Native Apache Hudi table format support
- **ACID Transactions**: ACID-compliant data operations
- **Incremental Processing**: Efficient incremental data processing
- **Schema Evolution**: Automatic schema evolution and compatibility
- **Time Travel**: Point-in-time data access and recovery
- **Optimized Storage**: Columnar storage with compression
- **Real-time Updates**: Real-time data ingestion and updates

## Key Features

### Hudi Table Support
- **Copy-on-Write (CoW)**: Snapshot isolation for read-heavy workloads
- **Merge-on-Read (MoR)**: Optimized for write-heavy workloads
- **Table Types**: Support for all Hudi table types
- **Partitioning**: Flexible partitioning strategies
- **Indexing**: Built-in indexing for fast queries

### Data Operations
- **Upsert Operations**: Efficient upsert with deduplication
- **Delete Operations**: Soft and hard delete support
- **Bulk Operations**: Bulk insert and update operations
- **Streaming Operations**: Real-time streaming data ingestion
- **Batch Operations**: Batch data processing and optimization

### Schema Management
- **Schema Evolution**: Automatic schema evolution handling
- **Backward Compatibility**: Backward-compatible schema changes
- **Schema Validation**: Schema validation and enforcement
- **Schema Registry**: Integration with schema registry
- **Type Mapping**: Automatic type mapping and conversion

### Performance Optimization
- **Compression**: Multiple compression algorithms support
- **Partitioning**: Intelligent partitioning strategies
- **Clustering**: Data clustering for query optimization
- **Indexing**: Secondary indexing for fast lookups
- **Caching**: Intelligent caching strategies

## Quick Start

### Basic Usage

```rust
use hudi_connector::{
    HudiConnector, HudiConfig, HudiTable,
    operations::{UpsertOperation, DeleteOperation},
    schema::HudiSchema,
};

#[tokio::main]
async fn main() -> hudi_connector::Result<()> {
    // Configure Hudi connector
    let config = HudiConfig::new()
        .with_warehouse_path("/path/to/warehouse")
        .with_table_name("telemetry_data")
        .with_table_type("COPY_ON_WRITE")
        .with_partition_fields(vec!["service_name", "date"])
        .with_record_key_field("id")
        .with_precombine_field("timestamp");
    
    let connector = HudiConnector::new(config);
    
    // Create or get table
    let table = connector.get_or_create_table().await?;
    
    // Define schema
    let schema = HudiSchema::new()
        .with_field("id", "string", true)
        .with_field("timestamp", "timestamp", false)
        .with_field("service_name", "string", false)
        .with_field("duration_ms", "long", false)
        .with_field("status_code", "int", false);
    
    // Upsert data
    let upsert_op = UpsertOperation::new()
        .with_records(vec![
            serde_json::json!({
                "id": "span_123",
                "timestamp": "2024-01-01T00:00:00Z",
                "service_name": "web-service",
                "duration_ms": 150,
                "status_code": 200
            })
        ]);
    
    let result = table.upsert(upsert_op).await?;
    println!("Upserted {} records", result.upserted_count);
    
    // Query data
    let query_result = table.query()
        .filter("service_name = 'web-service'")
        .select(vec!["id", "timestamp", "duration_ms"])
        .execute()
        .await?;
    
    for record in query_result.records {
        println!("Record: {:?}", record);
    }
    
    Ok(())
}
```

### Advanced Configuration

```rust
use hudi_connector::{
    HudiConnector, HudiConfig, HudiTable,
    operations::{BulkInsertOperation, IncrementalQueryOperation},
    schema::HudiSchema,
};

#[tokio::main]
async fn main() -> hudi_connector::Result<()> {
    // Advanced configuration
    let config = HudiConfig::new()
        .with_warehouse_path("/path/to/warehouse")
        .with_table_name("telemetry_data")
        .with_table_type("MERGE_ON_READ")
        .with_partition_fields(vec!["service_name", "date", "hour"])
        .with_record_key_field("id")
        .with_precombine_field("timestamp")
        .with_compression("SNAPPY")
        .with_parallelism(4)
        .with_batch_size(10000)
        .with_cleaner_enabled(true)
        .with_compaction_enabled(true);
    
    let connector = HudiConnector::new(config);
    let table = connector.get_or_create_table().await?;
    
    // Bulk insert operation
    let bulk_op = BulkInsertOperation::new()
        .with_records(generate_telemetry_data())
        .with_partition_path("service_name=web-service/date=2024-01-01");
    
    let result = table.bulk_insert(bulk_op).await?;
    println!("Bulk inserted {} records", result.inserted_count);
    
    // Incremental query
    let incremental_query = IncrementalQueryOperation::new()
        .with_begin_instant("20240101000000")
        .with_end_instant("20240102000000")
        .with_partition_path("service_name=web-service");
    
    let query_result = table.incremental_query(incremental_query).await?;
    println!("Incremental query returned {} records", query_result.records.len());
    
    Ok(())
}

fn generate_telemetry_data() -> Vec<serde_json::Value> {
    // Generate sample telemetry data
    (0..1000).map(|i| {
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
# hudi.toml
[hudi]
warehouse_path = "/data/warehouse"
table_name = "telemetry_data"
table_type = "COPY_ON_WRITE"  # COPY_ON_WRITE, MERGE_ON_READ

[hudi.table]
record_key_field = "id"
precombine_field = "timestamp"
partition_fields = ["service_name", "date", "hour"]
compression = "SNAPPY"
parallelism = 4
batch_size = 10000

[hudi.operations]
cleaner_enabled = true
cleaner_retain_commits = 10
compaction_enabled = true
compaction_schedule = "0 0 * * *"  # Daily at midnight

[hudi.indexing]
index_type = "BLOOM"  # BLOOM, GLOBAL_BLOOM, SIMPLE
bloom_filter_fpp = 0.01
bloom_filter_num_entries = 1000000

[hudi.schema]
auto_evolution = true
strict_mode = false
allow_nullable = true

[hudi.storage]
file_size_target = 134217728  # 128MB
file_size_max = 268435456     # 256MB
```

## API Reference

### HudiConnector

```rust
pub struct HudiConnector {
    config: HudiConfig,
}

impl HudiConnector {
    // Create new connector
    pub fn new(config: HudiConfig) -> Self;
    
    // Get or create table
    pub async fn get_or_create_table(&self) -> Result<HudiTable>;
    
    // Get existing table
    pub async fn get_table(&self) -> Result<HudiTable>;
    
    // List tables
    pub async fn list_tables(&self) -> Result<Vec<String>>;
    
    // Delete table
    pub async fn delete_table(&self) -> Result<()>;
}
```

### HudiTable

```rust
pub struct HudiTable {
    name: String,
    config: HudiConfig,
}

impl HudiTable {
    // Upsert operation
    pub async fn upsert(&self, operation: UpsertOperation) -> Result<UpsertResult>;
    
    // Delete operation
    pub async fn delete(&self, operation: DeleteOperation) -> Result<DeleteResult>;
    
    // Bulk insert operation
    pub async fn bulk_insert(&self, operation: BulkInsertOperation) -> Result<BulkInsertResult>;
    
    // Query operation
    pub fn query(&self) -> QueryBuilder;
    
    // Incremental query
    pub async fn incremental_query(&self, operation: IncrementalQueryOperation) -> Result<QueryResult>;
    
    // Get table metadata
    pub async fn get_metadata(&self) -> Result<TableMetadata>;
    
    // Get table statistics
    pub async fn get_statistics(&self) -> Result<TableStatistics>;
}
```

### Operations

#### UpsertOperation

```rust
pub struct UpsertOperation {
    records: Vec<serde_json::Value>,
    partition_path: Option<String>,
    deduplication: bool,
}

impl UpsertOperation {
    pub fn new() -> Self;
    pub fn with_records(mut self, records: Vec<serde_json::Value>) -> Self;
    pub fn with_partition_path(mut self, path: String) -> Self;
    pub fn with_deduplication(mut self, enabled: bool) -> Self;
}
```

#### DeleteOperation

```rust
pub struct DeleteOperation {
    keys: Vec<String>,
    partition_path: Option<String>,
    hard_delete: bool,
}

impl DeleteOperation {
    pub fn new() -> Self;
    pub fn with_keys(mut self, keys: Vec<String>) -> Self;
    pub fn with_partition_path(mut self, path: String) -> Self;
    pub fn with_hard_delete(mut self, enabled: bool) -> Self;
}
```

#### QueryBuilder

```rust
pub struct QueryBuilder {
    filters: Vec<String>,
    select_fields: Vec<String>,
    limit: Option<usize>,
    order_by: Option<String>,
}

impl QueryBuilder {
    pub fn filter(mut self, filter: &str) -> Self;
    pub fn select(mut self, fields: Vec<&str>) -> Self;
    pub fn limit(mut self, limit: usize) -> Self;
    pub fn order_by(mut self, field: &str) -> Self;
    pub async fn execute(self) -> Result<QueryResult>;
}
```

## Schema Management

### Schema Definition

```rust
use hudi_connector::schema::{HudiSchema, HudiField, HudiFieldType};

let schema = HudiSchema::new()
    .with_field("id", HudiFieldType::String, true)
    .with_field("timestamp", HudiFieldType::Timestamp, false)
    .with_field("service_name", HudiFieldType::String, false)
    .with_field("duration_ms", HudiFieldType::Long, false)
    .with_field("status_code", HudiFieldType::Int, false)
    .with_field("attributes", HudiFieldType::Map, false);
```

### Schema Evolution

```rust
// Add new field
let evolved_schema = schema
    .with_field("user_id", HudiFieldType::String, false)
    .with_field("session_id", HudiFieldType::String, false);

// Update table schema
table.update_schema(evolved_schema).await?;
```

## Performance Optimization

### Partitioning Strategies

```rust
// Time-based partitioning
let config = HudiConfig::new()
    .with_partition_fields(vec!["date", "hour", "service_name"]);

// Hash-based partitioning
let config = HudiConfig::new()
    .with_partition_fields(vec!["service_name"])
    .with_partition_strategy("hash");
```

### Compression and Storage

```rust
// Optimize for read performance
let config = HudiConfig::new()
    .with_compression("SNAPPY")
    .with_file_size_target(134217728)  // 128MB
    .with_parallelism(8);

// Optimize for write performance
let config = HudiConfig::new()
    .with_compression("GZIP")
    .with_file_size_target(67108864)   // 64MB
    .with_parallelism(4);
```

### Indexing

```rust
// Bloom filter indexing
let config = HudiConfig::new()
    .with_index_type("BLOOM")
    .with_bloom_filter_fpp(0.01)
    .with_bloom_filter_num_entries(1000000);

// Global bloom filter
let config = HudiConfig::new()
    .with_index_type("GLOBAL_BLOOM")
    .with_bloom_filter_fpp(0.001);
```

## Monitoring and Maintenance

### Table Maintenance

```rust
// Run compaction
table.compact().await?;

// Run cleaning
table.clean().await?;

// Get table statistics
let stats = table.get_statistics().await?;
println!("Table size: {} bytes", stats.total_size);
println!("File count: {}", stats.file_count);
```

### Monitoring Metrics

```rust
// Get operation metrics
let metrics = table.get_operation_metrics().await?;
println!("Upsert latency: {:?}", metrics.upsert_latency);
println!("Query latency: {:?}", metrics.query_latency);
println!("Compaction latency: {:?}", metrics.compaction_latency);
```

## Error Handling

```rust
use hudi_connector::error::HudiError;

match table.upsert(operation).await {
    Ok(result) => {
        println!("Upsert successful: {} records", result.upserted_count);
    }
    Err(HudiError::SchemaMismatch { expected, actual }) => {
        eprintln!("Schema mismatch: expected {:?}, got {:?}", expected, actual);
    }
    Err(HudiError::PartitionNotFound { partition }) => {
        eprintln!("Partition not found: {}", partition);
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

## Development

### Building

```bash
# Build connector
cargo build

# Build with optimizations
cargo build --release

# Build with features
cargo build --features hudi-native
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_upsert_operation

# Run integration tests
cargo test --test integration

# Run with logging
RUST_LOG=debug cargo test
```

### Examples

```bash
# Run examples
cargo run --example basic_usage
cargo run --example advanced_operations
cargo run --example schema_evolution
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
