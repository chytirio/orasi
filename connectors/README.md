# Connectors

This directory contains data connectors for integrating with various data sources, formats, and platforms in the Orasi observability platform.

## 🔌 Available Connectors

### Feature Matrix

<sub>**Legend**: ✅ Yes, ❌ No, 🚧 In Progress</sub>

| Name | Supported |               ||| Additional Features |
|:-----|:---------:|:-------:|:-:|:-:|:-------------------:|
| | | **Transactions** | **Schema Evolution** | **Time Travel** | |
| [Delta Lake](deltalake/README.md) | ✅ | ACID | ✅ | ✅ | Partition management, Upserts, CDC |
| [Apache Hudi](hudi/README.md) | 🚧 | ACID | ✅ | ✅ | Incremental processing, Compaction, Indexing |
| [Apache Iceberg](iceberg/README.md) | ✅ | ACID | ✅ | ✅ | Partition evolution, Metadata management |
| [Apache Kafka](kafka/README.md) | 🚧 | At-least-once | ❌ | ❌ | Topic management, Partition handling, Schema registry |
| [S3 Parquet](s3-parquet/README.md) | 🚧 | None | ❌ | ❌ | Compression, Partitioned writes, Metadata |
| [Snowflake](snowflake/README.md) | 🚧 | ACID | ✅ | ✅ | SQL queries, Bulk loading, Connection pooling |

## 🏗️ Architecture

### Connector Interface

All connectors implement a common interface for consistent usage:

```rust
pub trait Connector {
    type Config;
    type Error;
    
    async fn connect(&mut self) -> Result<(), Self::Error>;
    async fn disconnect(&mut self) -> Result<(), Self::Error>;
    async fn health_check(&self) -> Result<HealthStatus, Self::Error>;
}
```

### Configuration

Each connector supports configuration through environment variables and configuration files:

```toml
[connectors.deltalake]
table_path = "/path/to/table"
storage_options = { "fs.s3a.access.key" = "access_key" }

[connectors.kafka]
bootstrap_servers = ["localhost:9092"]
topic = "telemetry-data"
```

### Error Handling

Connectors provide comprehensive error handling with detailed error types:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Data validation failed: {0}")]
    ValidationFailed(String),
}
```

## 🚀 Quick Start

### Prerequisites

- Rust toolchain
- Target data source access credentials
- Required dependencies (Kafka, Delta Lake, etc.)

### Basic Usage

```rust
use orasi_connectors_deltalake::DeltaLakeConnector;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = DeltaLakeConfig::from_env()?;
    let mut connector = DeltaLakeConnector::new(config)?;
    
    connector.connect().await?;
    
    // Write data
    let data = vec![/* your data */];
    connector.write_batch(data).await?;
    
    connector.disconnect().await?;
    Ok(())
}
```

### Configuration

Set up environment variables for your connectors:

```bash
# Delta Lake
export DELTALAKE_TABLE_PATH="/path/to/table"
export DELTALAKE_STORAGE_OPTIONS='{"fs.s3a.access.key":"key"}'

# Kafka
export KAFKA_BOOTSTRAP_SERVERS="localhost:9092"
export KAFKA_TOPIC="telemetry-data"

# Snowflake
export SNOWFLAKE_ACCOUNT="your-account"
export SNOWFLAKE_USER="your-user"
export SNOWFLAKE_PASSWORD="your-password"
```

## 🧪 Testing

### Unit Tests

Each connector includes comprehensive unit tests:

```bash
# Run all connector tests
cargo test --package orasi-connectors-*

# Run specific connector tests
cargo test --package orasi-connectors-deltalake
```

### Integration Tests

Integration tests require running instances of the target systems:

```bash
# Start test environment
just delta-up

# Run integration tests
cargo test --package orasi-connectors-* --test integration

# Clean up
just delta-down-clean
```

### Test Data Generation

Use the data generator for creating test datasets:

```bash
cargo run -p data-generator -- --connector deltalake --records 1000
```

## 📊 Performance

### Optimization Tips

1. **Connection Pooling**: Reuse connections when possible
2. **Batch Operations**: Use batch writes for better throughput
3. **Parallel Processing**: Process multiple partitions in parallel
4. **Compression**: Enable appropriate compression for your data
5. **Caching**: Cache frequently accessed metadata

### Benchmarks

Performance benchmarks are available for each connector:

```bash
# Run performance benchmarks
cargo bench --package orasi-connectors-*

# Run specific connector benchmarks
cargo bench --package orasi-connectors-deltalake
```

## 🔒 Security

### Authentication

Each connector supports appropriate authentication methods:

- **Delta Lake**: AWS IAM, Azure AD, GCP IAM
- **Kafka**: SASL/PLAIN, SASL/SCRAM, SSL
- **Snowflake**: Username/password, OAuth, key pair
- **S3**: Access keys, IAM roles, temporary credentials

### Encryption

- **In-transit**: TLS/SSL encryption for all connections
- **At-rest**: Encryption support for stored data
- **Secrets**: Secure credential management

## 🔧 Configuration Reference

### Common Configuration Options

```toml
[connector]
# Connection settings
host = "localhost"
port = 8080
timeout_seconds = 30

# Authentication
username = "user"
password = "pass"

# Performance
batch_size = 1000
max_connections = 10
retry_attempts = 3

# Monitoring
enable_metrics = true
enable_tracing = true
```

### Environment Variables

All connectors support configuration via environment variables:

```bash
# Common patterns
export {CONNECTOR}_HOST="localhost"
export {CONNECTOR}_PORT="8080"
export {CONNECTOR}_USERNAME="user"
export {CONNECTOR}_PASSWORD="pass"
```

## 📚 Documentation

- [Delta Lake Connector](deltalake/README.md)
- [Kafka Connector](kafka/README.md)
- [Snowflake Connector](snowflake/README.md)
- [Testing Guide](tests/README.md)

## 🤝 Contributing

When adding new connectors:

1. **Follow the interface**: Implement the common `Connector` trait
2. **Add comprehensive tests**: Include unit and integration tests
3. **Document configuration**: Provide clear configuration examples
4. **Handle errors gracefully**: Implement proper error handling
5. **Add benchmarks**: Include performance benchmarks

## 📄 License

This project is licensed under the same terms as the main Orasi project.
