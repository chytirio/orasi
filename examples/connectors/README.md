# Connectors Examples

Examples for various data lake and storage connectors in the Orasi platform.

## Examples

### `delta_lake_example.rs`
Delta Lake integration showing how to work with Delta Lake storage.

### `real_delta_lake_example.rs`
Production Delta Lake setup with real-world configurations and patterns.

### `hudi_example.rs`
Apache Hudi integration demonstrating Hudi table operations.

### `iceberg_example.rs`
Apache Iceberg integration showing Iceberg table management.

### `s3_parquet_example.rs`
S3 Parquet storage examples for cloud-based data storage.

### `snowflake_example.rs`
Snowflake data warehouse integration patterns.

### `kafka_connector_example.rs`
Kafka connector usage showing how to connect to Kafka clusters.

### `kafka_example.rs`
Kafka integration patterns for streaming data.

## Running

```bash
cargo run --example delta_lake_example
cargo run --example real_delta_lake_example
cargo run --example hudi_example
cargo run --example iceberg_example
cargo run --example s3_parquet_example
cargo run --example snowflake_example
cargo run --example kafka_connector_example
cargo run --example kafka_example
```

## Prerequisites

Most examples require specific infrastructure:
- **Delta Lake**: Delta Lake storage (local or cloud)
- **Hudi**: Apache Hudi cluster
- **Iceberg**: Apache Iceberg catalog
- **S3**: AWS S3 bucket with appropriate permissions
- **Snowflake**: Snowflake account and credentials
- **Kafka**: Kafka cluster (local or remote)

Check individual example files for specific configuration requirements.
