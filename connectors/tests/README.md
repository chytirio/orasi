# Lake House Connectors Test Suite

This directory contains comprehensive unit and integration tests for all lake house connectors in the OpenTelemetry Data Lake Bridge.

## Test Structure

Each connector has the following test structure:

```
src/tests/
├── mod.rs          # Main test module with utilities
├── unit/           # Unit tests for individual components
├── integration/    # Integration tests for full workflows
└── common/         # Shared test utilities and fixtures
```

## Available Connectors

### 1. Delta Lake Connector (`deltalake/`)
- **Unit Tests**: Configuration, error handling, schema management, writer, reader, connector
- **Integration Tests**: End-to-end workflows, batch operations, concurrent operations, error handling
- **Dependencies**: Local file system (no external dependencies for unit tests)

### 2. Apache Kafka Connector (`kafka/`)
- **Unit Tests**: Configuration, error handling, topic management, producer, consumer, connector
- **Integration Tests**: Message streaming, consumer group coordination, topic management
- **Dependencies**: Kafka cluster (optional, can be skipped)

### 3. Apache Hudi Connector (`hudi/`)
- **Unit Tests**: Configuration, error handling, table management, writer, reader, connector
- **Integration Tests**: Table operations, incremental processing, data consistency
- **Dependencies**: Hudi runtime (optional, can be skipped)

### 4. Apache Iceberg Connector (`iceberg/`)
- **Unit Tests**: Configuration, error handling, catalog management, writer, reader, connector
- **Integration Tests**: Catalog operations, schema evolution, data consistency
- **Dependencies**: Iceberg catalog (optional, can be skipped)

### 5. Snowflake Connector (`snowflake/`)
- **Unit Tests**: Configuration, error handling, warehouse management, writer, reader, connector
- **Integration Tests**: Warehouse operations, query performance, data consistency
- **Dependencies**: Snowflake account (optional, can be skipped)

### 6. S3/Parquet Connector (`s3-parquet/`)
- **Unit Tests**: Configuration, error handling, storage management, writer, reader, connector
- **Integration Tests**: S3 operations, Parquet file handling, data consistency
- **Dependencies**: AWS S3 (optional, can be skipped)

## Running Tests

### Prerequisites

1. **Rust Toolchain**: Ensure you have Rust 1.70+ installed
2. **Dependencies**: Install required system dependencies for specific connectors

### Basic Test Execution

```bash
# Run all tests
cargo test

# Run tests for a specific connector
cargo test -p deltalake-connector
cargo test -p kafka-connector
cargo test -p hudi-connector
cargo test -p iceberg-connector
cargo test -p snowflake-connector
cargo test -p s3-parquet-connector

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration

# Run tests with output
cargo test -- --nocapture

# Run tests in parallel
cargo test -- --test-threads=4
```

### Environment Configuration

#### Kafka Tests
```bash
export KAFKA_TEST_BOOTSTRAP_SERVERS=localhost:9092
export KAFKA_TEST_TOPIC=test_telemetry
export KAFKA_TEST_GROUP_ID=test-consumer-group
```

#### Hudi Tests
```bash
export HUDI_TEST_TABLE_PATH=/tmp/test_hudi_table
export HUDI_TEST_TABLE_NAME=test_telemetry
```

#### Iceberg Tests
```bash
export ICEBERG_TEST_CATALOG_URI=file:///tmp/iceberg_catalog
export ICEBERG_TEST_WAREHOUSE_PATH=/tmp/iceberg_warehouse
```

#### Snowflake Tests
```bash
export SNOWFLAKE_TEST_ACCOUNT=your_account
export SNOWFLAKE_TEST_USERNAME=your_username
export SNOWFLAKE_TEST_PASSWORD=your_password
export SNOWFLAKE_TEST_WAREHOUSE=test_warehouse
export SNOWFLAKE_TEST_DATABASE=test_database
```

#### S3/Parquet Tests
```bash
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_DEFAULT_REGION=us-east-1
export S3_TEST_BUCKET=test-bucket
export S3_TEST_PREFIX=test/telemetry/
```

### Skipping External Dependencies

To skip tests that require external dependencies:

```bash
# Skip Kafka tests
export SKIP_KAFKA_TESTS=1

# Skip Hudi tests
export SKIP_HUDI_TESTS=1

# Skip Iceberg tests
export SKIP_ICEBERG_TESTS=1

# Skip Snowflake tests
export SKIP_SNOWFLAKE_TESTS=1

# Skip S3 tests
export SKIP_S3_TESTS=1
```

## Test Categories

### Unit Tests

Unit tests focus on individual components and functions:

- **Configuration Tests**: Validate configuration creation, validation, and defaults
- **Error Handling Tests**: Test error types, categorization, and retry logic
- **Schema Management Tests**: Test schema creation, validation, and evolution
- **Writer Tests**: Test data writing operations and batch processing
- **Reader Tests**: Test data reading operations and query processing
- **Connector Tests**: Test connector lifecycle and management

### Integration Tests

Integration tests focus on complete workflows:

- **End-to-End Tests**: Full telemetry data pipeline from write to read
- **Batch Operations**: Large-scale data processing and batch handling
- **Concurrent Operations**: Multi-threaded and concurrent access patterns
- **Error Handling**: Error recovery and resilience testing
- **Performance Tests**: Throughput and latency benchmarking
- **Data Consistency**: Data integrity and consistency validation

### Common Test Utilities

Shared utilities for all connectors:

- **Data Generators**: Create test telemetry data (metrics, traces, logs)
- **Query Generators**: Create test queries for different telemetry types
- **Assertions**: Validate data structures and operation results
- **Performance Utilities**: Measure and benchmark operation performance
- **Error Testing**: Test error conditions and recovery scenarios

## Test Data

### Telemetry Data Types

All connectors support the following telemetry data types:

1. **Metrics**: Time-series numerical data with labels
2. **Traces**: Distributed tracing data with spans and attributes
3. **Logs**: Structured log data with levels and attributes
4. **Mixed Batches**: Combined telemetry data for batch processing

### Test Scenarios

Common test scenarios across all connectors:

1. **Basic Operations**: Simple write and read operations
2. **Batch Processing**: Large batch handling and optimization
3. **Concurrent Access**: Multi-threaded and concurrent operations
4. **Error Recovery**: Network failures, timeouts, and retry logic
5. **Schema Evolution**: Schema changes and backward compatibility
6. **Performance**: Throughput, latency, and resource usage
7. **Data Consistency**: ACID properties and data integrity

## Continuous Integration

### GitHub Actions

The test suite is integrated with GitHub Actions:

```yaml
name: Test Lake House Connectors

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        connector: [deltalake, kafka, hudi, iceberg, snowflake, s3-parquet]
    
    steps:
    - uses: actions/checkout@v3
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    
    - name: Run unit tests
      run: cargo test -p ${{ matrix.connector }}-connector --lib
    
    - name: Run integration tests (if dependencies available)
      run: |
        if [ "${{ matrix.connector }}" = "deltalake" ]; then
          cargo test -p ${{ matrix.connector }}-connector --test integration
        fi
      continue-on-error: true
```

### Local Development

For local development, use the following workflow:

1. **Unit Tests**: Always run unit tests (no external dependencies)
2. **Integration Tests**: Run integration tests when external services are available
3. **Performance Tests**: Run performance tests for benchmarking
4. **Manual Testing**: Test with real data and scenarios

## Troubleshooting

### Common Issues

1. **Connection Failures**: Check external service availability and credentials
2. **Timeout Errors**: Increase timeout values for slow operations
3. **Resource Limits**: Ensure sufficient memory and disk space
4. **Permission Errors**: Check file system and network permissions

### Debug Mode

Enable debug logging for troubleshooting:

```bash
export RUST_LOG=debug
cargo test -- --nocapture
```

### Test Isolation

Each test should be isolated and not depend on other tests:

- Use unique identifiers for test data
- Clean up resources after each test
- Use temporary directories and databases
- Reset state between tests

## Contributing

When adding new tests:

1. **Follow the existing structure** for consistency
2. **Add both unit and integration tests** for new features
3. **Use shared utilities** from the common module
4. **Document test requirements** and dependencies
5. **Ensure test isolation** and cleanup
6. **Add performance benchmarks** for critical paths

## Test Coverage

Current test coverage targets:

- **Unit Tests**: >90% line coverage
- **Integration Tests**: All major workflows covered
- **Error Scenarios**: All error types and recovery paths
- **Performance**: Baseline performance benchmarks
- **Data Consistency**: ACID property validation
