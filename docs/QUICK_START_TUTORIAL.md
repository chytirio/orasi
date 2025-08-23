# Quick Start Tutorial

This tutorial will guide you through setting up and using Orasi for the first time. By the end of this tutorial, you'll have a working Orasi instance that can ingest, process, and query telemetry data.

## Prerequisites

- Rust 1.70+ installed
- Docker (optional, for running external services)
- Basic understanding of OpenTelemetry concepts

## Step 1: Installation

### Clone the Repository
```bash
git clone https://github.com/chytirio/orasi.git
cd orasi
```

### Build the Project
```bash
# Build all components
cargo build --release

# Verify the build
cargo test
```

## Step 2: Basic Configuration

### Create Configuration File
Create a basic configuration file at `config/bridge.toml`:

```toml
# Basic Orasi Configuration
name = "orasi-tutorial"
version = "0.1.0"
environment = "development"

# API Configuration
[api]
host = "0.0.0.0"
port = 8080

# gRPC Configuration
[grpc]
host = "0.0.0.0"
port = 9090

# Ingestion Configuration
[ingestion]
otlp_endpoint = "0.0.0.0:4317"
http_endpoint = "0.0.0.0:4318"
batch_size = 1000
flush_interval_ms = 5000

# Processing Configuration
[processing]
worker_threads = 4
enable_streaming = true
enable_transformation = true
enable_filtering = true

# Query Engine Configuration
[query_engine]
worker_threads = 8
query_timeout_seconds = 60
enable_query_caching = true

# Schema Registry Configuration
[schema_registry]
host = "0.0.0.0"
port = 8081
storage = "memory"
```

## Step 3: Start Core Services

### Start the Bridge API
```bash
# Terminal 1: Start the bridge API server
cargo run --bin bridge-api -- --config config/bridge.toml
```

You should see output like:
```
[INFO] Starting Orasi Bridge API server
[INFO] Listening on http://0.0.0.0:8080
[INFO] gRPC server listening on 0.0.0.0:9090
[INFO] Bridge API server started successfully
```

### Start the Schema Registry
```bash
# Terminal 2: Start the schema registry
cargo run --bin schema-registry -- --config config/schema-registry.toml
```

You should see output like:
```
[INFO] Starting Schema Registry server
[INFO] Listening on http://0.0.0.0:8081
[INFO] Schema Registry server started successfully
```

## Step 4: Send Test Data

### Using the Test Data Generator
```bash
# Terminal 3: Generate and send test data
cargo run --bin test-data-generator -- --config config/generator.toml
```

### Using curl (Alternative)
```bash
# Send a simple metric via HTTP
curl -X POST http://localhost:4318/v1/metrics \
  -H "Content-Type: application/json" \
  -d '{
    "resourceMetrics": [{
      "resource": {
        "attributes": [{
          "key": "service.name",
          "value": { "stringValue": "tutorial-service" }
        }]
      },
      "scopeMetrics": [{
        "metrics": [{
          "name": "http_requests_total",
          "unit": "requests",
          "sum": {
            "dataPoints": [{
              "timeUnixNano": "1640995200000000000",
              "value": 42
            }]
          }
        }]
      }]
    }]
  }'
```

## Step 5: Query Your Data

### Using the Query Engine
```bash
# Terminal 4: Start the query engine
cargo run --bin query-engine -- --config config/query-engine.toml
```

### Run a Simple Query
```bash
# Query your metrics
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "SELECT * FROM metrics WHERE service_name = '\''tutorial-service'\''",
    "format": "json"
  }'
```

## Step 6: Monitor Your System

### Check Health Endpoints
```bash
# Check API health
curl http://localhost:8080/health

# Check detailed health
curl http://localhost:8080/health/detailed

# Check metrics
curl http://localhost:9090/metrics
```

### View Logs
The services will output structured logs. Look for:
- Connection information
- Data processing statistics
- Error messages (if any)

## Step 7: Advanced Configuration

### Add a Data Lake Connector
Create `config/delta-lake.toml`:

```toml
[delta_lake]
storage_path = "file:///tmp/orasi-data"
table_format_version = 2
enable_transactions = true
partition_columns = ["service_name", "date", "hour"]
compression = "zstd"
```

### Update Bridge Configuration
Add to your `config/bridge.toml`:

```toml
# Add Delta Lake configuration
[lakehouses.delta_lake]
type = "DeltaLake"
storage_path = "file:///tmp/orasi-data"
catalog = "hive"
table_format_version = 2
enable_transactions = true
partition_columns = ["service_name", "date", "hour"]
compression = "zstd"
```

## Step 8: Process Real Data

### Create a Custom Processor
Create `examples/custom_processor.rs`:

```rust
use orasi_ingestion::{
    processors::{Processor, ProcessorConfig},
    types::TelemetryRecord,
};

pub struct CustomProcessor {
    config: ProcessorConfig,
}

impl CustomProcessor {
    pub fn new(config: ProcessorConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl Processor for CustomProcessor {
    async fn process(&self, records: Vec<TelemetryRecord>) -> Result<Vec<TelemetryRecord>, Box<dyn std::error::Error>> {
        // Add your custom processing logic here
        let processed_records = records.into_iter()
            .map(|record| {
                // Example: Add a custom attribute
                let mut record = record;
                record.attributes.insert("processed_by".to_string(), "custom_processor".to_string());
                record
            })
            .collect();
        
        Ok(processed_records)
    }
}
```

### Run with Custom Processor
```bash
cargo run --example custom_processor
```

## Step 9: Scale Your Setup

### Run Multiple Instances
```bash
# Terminal 1: Instance 1
RUST_LOG=info cargo run --bin bridge-api -- --config config/bridge-1.toml

# Terminal 2: Instance 2
RUST_LOG=info cargo run --bin bridge-api -- --config config/bridge-2.toml

# Terminal 3: Load balancer (using nginx or similar)
```

### Monitor Performance
```bash
# Check throughput
curl http://localhost:8080/metrics | grep orasi_ingestion_records_total

# Check latency
curl http://localhost:8080/metrics | grep orasi_ingestion_latency_seconds
```

## Step 10: Troubleshooting

### Common Issues

#### Service Won't Start
```bash
# Check if ports are available
netstat -tulpn | grep :8080
netstat -tulpn | grep :9090

# Check configuration
cargo run --bin bridge-api -- --config config/bridge.toml --validate
```

#### No Data Received
```bash
# Check if ingestion endpoints are listening
curl http://localhost:4318/health

# Check logs for errors
RUST_LOG=debug cargo run --bin bridge-api
```

#### Query Returns No Results
```bash
# Check if data was ingested
curl http://localhost:8080/api/v1/query \
  -d '{"query": "SELECT COUNT(*) FROM metrics"}'

# Check schema registry
curl http://localhost:8081/schemas
```

### Debug Mode
```bash
# Run with debug logging
RUST_LOG=debug cargo run --bin bridge-api

# Run with specific log levels
RUST_LOG=orasi_bridge=debug,orasi_ingestion=info cargo run --bin bridge-api
```

## Next Steps

### Explore Examples
```bash
# Run all examples
just examples

# Run specific examples
cargo run --example basic_usage
cargo run --example delta_lake_example
cargo run --example streaming_processor_example
```

### Read Documentation
- [API Reference](API_REFERENCE.md)
- [Configuration Guide](CONFIGURATION.md)
- [Deployment Guide](DEPLOYMENT_GUIDE.md)
- [Development Setup](DEVELOPMENT_SETUP.md)

### Join the Community
- [GitHub Issues](https://github.com/chytirio/orasi/issues)
- [GitHub Discussions](https://github.com/chytirio/orasi/discussions)
- [Contributing Guide](CONTRIBUTING.md)

## Summary

In this tutorial, you've:
1. ✅ Installed and built Orasi
2. ✅ Created a basic configuration
3. ✅ Started core services
4. ✅ Sent test telemetry data
5. ✅ Queried your data
6. ✅ Monitored your system
7. ✅ Added advanced configuration
8. ✅ Created custom processors
9. ✅ Scaled your setup
10. ✅ Learned troubleshooting techniques

You now have a working Orasi instance that can:
- Ingest OpenTelemetry data via multiple protocols
- Process and transform telemetry data
- Store data in various lakehouse formats
- Query data using SQL
- Monitor system health and performance

## What's Next?

- **Production Deployment**: Follow the [Deployment Guide](DEPLOYMENT_GUIDE.md)
- **Custom Connectors**: Learn how to build custom data connectors
- **Advanced Analytics**: Explore streaming queries and ML integration
- **Contributing**: Help improve Orasi by contributing code or documentation

---

**Congratulations! You've successfully set up and used Orasi! 🎉**
