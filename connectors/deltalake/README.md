# Delta Lake Connector

Delta Lake connector for the Orasi OpenTelemetry Data Lake Bridge, providing ACID transactions, schema evolution, and time travel capabilities for telemetry data storage.

## Overview

The Delta Lake connector enables the Orasi bridge to store and manage OpenTelemetry telemetry data using Delta Lake format, offering:

- **ACID Transactions**: ACID-compliant data operations with optimistic concurrency control
- **Schema Evolution**: Automatic schema evolution and backward compatibility
- **Time Travel**: Point-in-time data access and data versioning
- **Upsert/Delete**: Efficient upsert and delete operations
- **Optimized Storage**: Columnar storage with compression and indexing
- **Streaming**: Real-time streaming data ingestion
- **Audit Trail**: Complete audit trail of all data changes

## Key Features

### ACID Transactions
- **Atomic Operations**: All-or-nothing data operations
- **Consistency**: Data consistency across concurrent operations
- **Isolation**: Transaction isolation with optimistic concurrency control
- **Durability**: Persistent data storage with write-ahead logging

### Schema Management
- **Schema Evolution**: Add, remove, and modify columns safely
- **Backward Compatibility**: Read data with older schemas
- **Schema Validation**: Automatic schema validation and enforcement
- **Type Mapping**: Automatic type mapping and conversion
- **Schema Registry**: Integration with schema registry

### Data Operations
- **Upsert Operations**: Efficient upsert with deduplication
- **Delete Operations**: Soft and hard delete support
- **Bulk Operations**: Bulk insert and update operations
- **Streaming Operations**: Real-time streaming data ingestion
- **Batch Operations**: Batch data processing and optimization

### Time Travel
- **Version History**: Complete version history of data changes
- **Point-in-time Queries**: Query data at specific points in time
- **Data Recovery**: Recover data from previous versions
- **Audit Trail**: Complete audit trail of all changes
- **Rollback**: Rollback to previous data versions

## Quick Start

### Basic Usage

```rust
use deltalake_connector::{
    DeltaLakeConnector, DeltaConfig, DeltaTable,
    operations::{UpsertOperation, DeleteOperation},
    schema::DeltaSchema,
};

#[tokio::main]
async fn main() -> deltalake_connector::Result<()> {
    // Configure Delta Lake connector
    let config = DeltaConfig::new()
        .with_table_path("/path/to/delta/table")
        .with_table_name("telemetry_data")
        .with_partition_by(vec!["service_name", "date"])
        .with_optimize_enabled(true)
        .with_vacuum_enabled(true);
    
    let connector = DeltaLakeConnector::new(config);
    
    // Create or get table
    let table = connector.get_or_create_table().await?;
    
    // Define schema
    let schema = DeltaSchema::new()
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
        ])
        .with_merge_key("id");
    
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
use deltalake_connector::{
    DeltaLakeConnector, DeltaConfig, DeltaTable,
    operations::{BulkInsertOperation, TimeTravelQueryOperation},
    schema::DeltaSchema,
};

#[tokio::main]
async fn main() -> deltalake_connector::Result<()> {
    // Advanced configuration
    let config = DeltaConfig::new()
        .with_table_path("/path/to/delta/table")
        .with_table_name("telemetry_data")
        .with_partition_by(vec!["service_name", "date", "hour"])
        .with_optimize_enabled(true)
        .with_vacuum_enabled(true)
        .with_retention_hours(168)  // 7 days
        .with_compression("SNAPPY")
        .with_parallelism(4)
        .with_batch_size(10000);
    
    let connector = DeltaLakeConnector::new(config);
    let table = connector.get_or_create_table().await?;
    
    // Bulk insert operation
    let bulk_op = BulkInsertOperation::new()
        .with_records(generate_telemetry_data())
        .with_partition_path("service_name=web-service/date=2024-01-01");
    
    let result = table.bulk_insert(bulk_op).await?;
    println!("Bulk inserted {} records", result.inserted_count);
    
    // Time travel query
    let time_travel_query = TimeTravelQueryOperation::new()
        .with_version(5)  // Query version 5
        .with_filter("service_name = 'web-service'");
    
    let query_result = table.time_travel_query(time_travel_query).await?;
    println!("Time travel query returned {} records", query_result.records.len());
    
    // Get table history
    let history = table.get_history().await?;
    for version in history.versions {
        println!("Version {}: {} records, timestamp: {}", 
                version.version, version.num_records, version.timestamp);
    }
    
    Ok(())
}

fn generate_telemetry_data() -> Vec<serde_json::Value> {
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
# deltalake.toml
[deltalake]
table_path = "/data/delta/telemetry_data"
table_name = "telemetry_data"
partition_by = ["service_name", "date", "hour"]

[deltalake.table]
optimize_enabled = true
vacuum_enabled = true
retention_hours = 168  # 7 days
compression = "SNAPPY"
parallelism = 4
batch_size = 10000

[deltalake.operations]
upsert_enabled = true
delete_enabled = true
bulk_insert_enabled = true
streaming_enabled = true

[deltalake.schema]
auto_evolution = true
strict_mode = false
allow_nullable = true
merge_schema = true

[deltalake.storage]
file_size_target = 134217728  # 128MB
file_size_max = 268435456     # 256MB
min_file_size = 67108864      # 64MB
```

## API Reference

### DeltaLakeConnector

```rust
pub struct DeltaLakeConnector {
    config: DeltaConfig,
}

impl DeltaLakeConnector {
    // Create new connector
    pub fn new(config: DeltaConfig) -> Self;
    
    // Get or create table
    pub async fn get_or_create_table(&self) -> Result<DeltaTable>;
    
    // Get existing table
    pub async fn get_table(&self) -> Result<DeltaTable>;
    
    // List tables
    pub async fn list_tables(&self) -> Result<Vec<String>>;
    
    // Delete table
    pub async fn delete_table(&self) -> Result<()>;
}
```

### DeltaTable

```rust
pub struct DeltaTable {
    name: String,
    config: DeltaConfig,
}

impl DeltaTable {
    // Upsert operation
    pub async fn upsert(&self, operation: UpsertOperation) -> Result<UpsertResult>;
    
    // Delete operation
    pub async fn delete(&self, operation: DeleteOperation) -> Result<DeleteResult>;
    
    // Bulk insert operation
    pub async fn bulk_insert(&self, operation: BulkInsertOperation) -> Result<BulkInsertResult>;
    
    // Query operation
    pub fn query(&self) -> QueryBuilder;
    
    // Time travel query
    pub async fn time_travel_query(&self, operation: TimeTravelQueryOperation) -> Result<QueryResult>;
    
    // Get table metadata
    pub async fn get_metadata(&self) -> Result<TableMetadata>;
    
    // Get table history
    pub async fn get_history(&self) -> Result<TableHistory>;
    
    // Optimize table
    pub async fn optimize(&self) -> Result<OptimizeResult>;
    
    // Vacuum table
    pub async fn vacuum(&self, retention_hours: Option<i64>) -> Result<VacuumResult>;
}
```

### Operations

#### UpsertOperation

```rust
pub struct UpsertOperation {
    records: Vec<serde_json::Value>,
    merge_key: String,
    partition_path: Option<String>,
    deduplication: bool,
}

impl UpsertOperation {
    pub fn new() -> Self;
    pub fn with_records(mut self, records: Vec<serde_json::Value>) -> Self;
    pub fn with_merge_key(mut self, key: String) -> Self;
    pub fn with_partition_path(mut self, path: String) -> Self;
    pub fn with_deduplication(mut self, enabled: bool) -> Self;
}
```

#### DeleteOperation

```rust
pub struct DeleteOperation {
    condition: String,
    partition_path: Option<String>,
    hard_delete: bool,
}

impl DeleteOperation {
    pub fn new() -> Self;
    pub fn with_condition(mut self, condition: String) -> Self;
    pub fn with_partition_path(mut self, path: String) -> Self;
    pub fn with_hard_delete(mut self, enabled: bool) -> Self;
}
```

#### TimeTravelQueryOperation

```rust
pub struct TimeTravelQueryOperation {
    version: Option<i64>,
    timestamp: Option<String>,
    filter: Option<String>,
    select_fields: Vec<String>,
}

impl TimeTravelQueryOperation {
    pub fn new() -> Self;
    pub fn with_version(mut self, version: i64) -> Self;
    pub fn with_timestamp(mut self, timestamp: String) -> Self;
    pub fn with_filter(mut self, filter: String) -> Self;
    pub fn with_select_fields(mut self, fields: Vec<String>) -> Self;
}
```

## Schema Management

### Schema Definition

```rust
use deltalake_connector::schema::{DeltaSchema, DeltaField, DeltaFieldType};

let schema = DeltaSchema::new()
    .with_field("id", DeltaFieldType::String, true)
    .with_field("timestamp", DeltaFieldType::Timestamp, false)
    .with_field("service_name", DeltaFieldType::String, false)
    .with_field("duration_ms", DeltaFieldType::Long, false)
    .with_field("status_code", DeltaFieldType::Int, false)
    .with_field("attributes", DeltaFieldType::Map, false);
```

### Schema Evolution

```rust
// Add new field
let evolved_schema = schema
    .with_field("user_id", DeltaFieldType::String, false)
    .with_field("session_id", DeltaFieldType::String, false);

// Update table schema
table.update_schema(evolved_schema).await?;
```

## Time Travel

### Version-based Time Travel

```rust
// Query specific version
let time_travel_query = TimeTravelQueryOperation::new()
    .with_version(5)
    .with_filter("service_name = 'web-service'");

let result = table.time_travel_query(time_travel_query).await?;
```

### Timestamp-based Time Travel

```rust
// Query data at specific timestamp
let time_travel_query = TimeTravelQueryOperation::new()
    .with_timestamp("2024-01-01T12:00:00Z")
    .with_filter("service_name = 'web-service'");

let result = table.time_travel_query(time_travel_query).await?;
```

### Table History

```rust
// Get table history
let history = table.get_history().await?;

for version in history.versions {
    println!("Version {}: {} records, timestamp: {}", 
            version.version, version.num_records, version.timestamp);
    println!("  Operation: {}", version.operation);
    println!("  User: {}", version.user);
}
```

## Performance Optimization

### Table Optimization

```rust
// Optimize table
let optimize_result = table.optimize().await?;
println!("Optimized {} files", optimize_result.files_optimized);
println!("Z-order columns: {:?}", optimize_result.z_order_columns);

// Vacuum table
let vacuum_result = table.vacuum(Some(168)).await?;  // 7 days retention
println!("Vacuumed {} files", vacuum_result.files_deleted);
```

### Partitioning Strategies

```rust
// Time-based partitioning
let config = DeltaConfig::new()
    .with_partition_by(vec!["date", "hour", "service_name"]);

// Hash-based partitioning
let config = DeltaConfig::new()
    .with_partition_by(vec!["service_name"])
    .with_partition_strategy("hash");
```

### Compression and Storage

```rust
// Optimize for read performance
let config = DeltaConfig::new()
    .with_compression("SNAPPY")
    .with_file_size_target(134217728)  // 128MB
    .with_parallelism(8);

// Optimize for write performance
let config = DeltaConfig::new()
    .with_compression("GZIP")
    .with_file_size_target(67108864)   // 64MB
    .with_parallelism(4);
```

## Monitoring and Maintenance

### Table Maintenance

```rust
// Get table statistics
let stats = table.get_statistics().await?;
println!("Table size: {} bytes", stats.total_size);
println!("File count: {}", stats.file_count);
println!("Partition count: {}", stats.partition_count);

// Get table metadata
let metadata = table.get_metadata().await?;
println!("Table version: {}", metadata.version);
println!("Schema: {:?}", metadata.schema);
println!("Partition columns: {:?}", metadata.partition_columns);
```

### Operation Metrics

```rust
// Get operation metrics
let metrics = table.get_operation_metrics().await?;
println!("Upsert latency: {:?}", metrics.upsert_latency);
println!("Query latency: {:?}", metrics.query_latency);
println!("Optimize latency: {:?}", metrics.optimize_latency);
```

## Error Handling

```rust
use deltalake_connector::error::DeltaLakeError;

match table.upsert(operation).await {
    Ok(result) => {
        println!("Upsert successful: {} records", result.upserted_count);
    }
    Err(DeltaLakeError::SchemaMismatch { expected, actual }) => {
        eprintln!("Schema mismatch: expected {:?}, got {:?}", expected, actual);
    }
    Err(DeltaLakeError::ConcurrentModification { version }) => {
        eprintln!("Concurrent modification detected at version {}", version);
    }
    Err(DeltaLakeError::PartitionNotFound { partition }) => {
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
cargo build --features deltalake-native
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
cargo run --example time_travel
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
