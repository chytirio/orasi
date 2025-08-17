# Testing

This directory contains comprehensive testing infrastructure for the Orasi observability platform, including data generators, test runners, and testing utilities.

## 🧪 Testing Components

### Data Generator

**Location**: `data-generator/`

A comprehensive tool for generating synthetic telemetry data for testing and benchmarking purposes.

**Features**:
- **Multiple Data Types**: Logs, metrics, traces, and events
- **Realistic Patterns**: Time-series data with realistic distributions
- **Customizable Schemas**: Configurable data structures and fields
- **High Performance**: Efficient generation for large datasets
- **Multiple Formats**: JSON, Parquet, Arrow, and Protocol Buffers
- **Export Options**: File, HTTP, Kafka, and database exports

**Usage**:
```bash
# Generate basic telemetry data
cargo run -p data-generator -- --records 1000

# Generate specific data types
cargo run -p data-generator -- --type metrics --records 10000

# Export to Delta Lake
cargo run -p data-generator -- --connector deltalake --records 50000

# Custom schema generation
cargo run -p data-generator -- --schema custom-schema.yaml --records 1000
```

### Test Runner

**Location**: `test-runner/`

An automated test execution framework for running comprehensive test suites across different environments and configurations.

**Features**:
- **Multi-Environment Testing**: Local, Docker, and Kubernetes environments
- **Performance Testing**: Load testing and benchmarking
- **Integration Testing**: End-to-end system testing
- **Regression Testing**: Automated regression detection
- **Test Reporting**: Comprehensive test reports and metrics

**Usage**:
```bash
# Run all tests
cargo run -p test-runner -- --suite all

# Run specific test categories
cargo run -p test-runner -- --suite integration --connector deltalake

# Performance testing
cargo run -p test-runner -- --suite performance --duration 300

# Generate test report
cargo run -p test-runner -- --suite all --report html
```

### Testing Scripts

**Location**: `scripts/`

Automation scripts for setting up testing environments and running common test scenarios.

**Available Scripts**:
- `setup-testing-platform.sh`: Sets up the complete testing environment
- `test-delta-lake.sh`: Runs Delta Lake-specific tests

**Usage**:
```bash
# Set up testing platform
./scripts/setup-testing-platform.sh

# Run Delta Lake tests
./scripts/test-delta-lake.sh
```

## 🚀 Quick Start

### Prerequisites

- Rust toolchain
- Docker and Docker Compose
- Kubernetes cluster (optional)
- Target data sources (Delta Lake, Kafka, etc.)

### Setup Testing Environment

```bash
# Clone and setup
git clone https://github.com/your-org/orasi.git
cd orasi

# Setup testing platform
./testing/scripts/setup-testing-platform.sh

# Start test infrastructure
just delta-up
```

### Generate Test Data

```bash
# Generate basic test data
cargo run -p data-generator -- --records 1000

# Generate data for specific connector
cargo run -p data-generator -- --connector deltalake --records 10000

# Generate performance test data
cargo run -p data-generator -- --type metrics --records 100000 --format parquet
```

### Run Tests

```bash
# Run all tests
cargo run -p test-runner -- --suite all

# Run integration tests
cargo run -p test-runner -- --suite integration

# Run performance tests
cargo run -p test-runner -- --suite performance --duration 600
```

## 📊 Test Categories

### Unit Tests

Individual component testing with mocked dependencies:

```bash
# Run all unit tests
cargo test

# Run specific package tests
cargo test --package bridge-core
cargo test --package bridge-api
```

### Integration Tests

End-to-end testing with real dependencies:

```bash
# Run integration tests
cargo test --package orasi-connectors-* --test integration

# Run with specific environment
cargo test --package orasi-connectors-deltalake --test integration -- --nocapture
```

### Performance Tests

Load testing and benchmarking:

```bash
# Run performance benchmarks
cargo bench

# Run load tests
cargo run -p test-runner -- --suite performance --duration 300 --concurrent 10
```

### End-to-End Tests

Complete system testing:

```bash
# Run E2E tests
cargo run -p test-runner -- --suite e2e --environment docker

# Run with custom configuration
cargo run -p test-runner -- --suite e2e --config test-config.yaml
```

## 🔧 Configuration

### Data Generator Configuration

```yaml
# config/data-generator.yaml
generator:
  records: 10000
  batch_size: 1000
  format: json
  
data_types:
  - logs
  - metrics
  - traces
  
schema:
  logs:
    fields:
      - name: timestamp
        type: datetime
      - name: level
        type: enum
        values: [debug, info, warn, error]
      - name: message
        type: string
        length: 100-500

export:
  type: file
  path: ./output/telemetry-data.json
```

### Test Runner Configuration

```yaml
# config/test-runner.yaml
suites:
  unit:
    enabled: true
    timeout: 300
    
  integration:
    enabled: true
    timeout: 600
    environment: docker
    
  performance:
    enabled: true
    duration: 300
    concurrent_users: 10
    
  e2e:
    enabled: true
    timeout: 1800
    environment: kubernetes

reporting:
  format: html
  output: ./test-reports/
  include_metrics: true
```

### Environment Variables

```bash
# Test configuration
export TEST_ENVIRONMENT="docker"
export TEST_TIMEOUT="600"
export TEST_CONCURRENT="5"

# Data generation
export DATA_GENERATOR_RECORDS="10000"
export DATA_GENERATOR_FORMAT="parquet"
export DATA_GENERATOR_BATCH_SIZE="1000"

# Test reporting
export TEST_REPORT_FORMAT="html"
export TEST_REPORT_OUTPUT="./reports/"
```

## 📈 Performance Testing

### Load Testing

```bash
# Basic load test
cargo run -p test-runner -- --suite performance --duration 300 --users 10

# Stress test
cargo run -p test-runner -- --suite performance --duration 600 --users 50 --ramp-up 60

# Spike test
cargo run -p test-runner -- --suite performance --duration 300 --users 100 --spike-duration 30
```

### Benchmarking

```bash
# Run benchmarks
cargo bench --package bridge-core
cargo bench --package bridge-api

# Compare benchmarks
cargo bench --package bridge-core -- --baseline previous
```

### Monitoring

Performance tests include comprehensive monitoring:

- **Resource Usage**: CPU, memory, disk I/O
- **Response Times**: P50, P95, P99 latencies
- **Throughput**: Requests per second
- **Error Rates**: Success/failure ratios

## 🐛 Debugging Tests

### Verbose Output

```bash
# Enable verbose logging
RUST_LOG=debug cargo test -- --nocapture

# Verbose test runner
cargo run -p test-runner -- --suite all --verbose
```

### Test Isolation

```bash
# Run single test
cargo test test_name -- --nocapture

# Run tests in isolation
cargo test --package bridge-api --test integration -- --test-threads=1
```

### Debug Data Generation

```bash
# Generate small dataset for debugging
cargo run -p data-generator -- --records 10 --verbose

# Validate generated data
cargo run -p data-generator -- --validate --records 100
```

## 📊 Test Reporting

### Report Formats

- **HTML**: Interactive web reports
- **JSON**: Machine-readable reports
- **JUnit**: CI/CD integration
- **Markdown**: Documentation-friendly

### Metrics Included

- **Test Results**: Pass/fail status
- **Performance**: Response times, throughput
- **Coverage**: Code coverage metrics
- **Trends**: Historical performance data

### Example Report

```bash
# Generate comprehensive report
cargo run -p test-runner -- --suite all --report html --output ./reports/

# View report
open ./reports/index.html
```

## 🔄 Continuous Integration

### GitHub Actions

```yaml
# .github/workflows/test.yml
name: Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Setup testing environment
        run: ./testing/scripts/setup-testing-platform.sh
      
      - name: Run tests
        run: cargo run -p test-runner -- --suite all --report junit
      
      - name: Upload test results
        uses: actions/upload-artifact@v3
        with:
          name: test-results
          path: test-reports/
```

### Local CI

```bash
# Run full CI locally
./testing/scripts/run-ci.sh

# Run specific CI stage
./testing/scripts/run-ci.sh --stage test
```

## 🧹 Test Maintenance

### Test Data Management

```bash
# Clean test data
cargo run -p test-runner -- --cleanup

# Reset test environment
./testing/scripts/reset-environment.sh
```

### Test Validation

```bash
# Validate test configuration
cargo run -p test-runner -- --validate-config

# Check test dependencies
cargo run -p test-runner -- --check-dependencies
```

## 📚 Documentation

- [Data Generator Documentation](data-generator/README.md)
- [Test Runner Documentation](test-runner/README.md)
- [Testing Best Practices](../docs/testing-best-practices.md)
- [Performance Testing Guide](../docs/performance-testing.md)

## 🤝 Contributing

When contributing to testing:

1. **Add tests for new features**: Include unit, integration, and performance tests
2. **Update test data**: Ensure test data covers new scenarios
3. **Maintain test performance**: Keep tests fast and reliable
4. **Document test scenarios**: Clear documentation for test cases
5. **Follow testing patterns**: Consistent test structure and naming

## 📄 License

This project is licensed under the same terms as the main Orasi project.
