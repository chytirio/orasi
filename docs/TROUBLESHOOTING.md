# Troubleshooting Guide

This guide helps you diagnose and resolve common issues with Orasi. If you can't find a solution here, please check the [GitHub Issues](https://github.com/chytirio/orasi/issues) or start a [GitHub Discussion](https://github.com/chytirio/orasi/discussions).

## Table of Contents

- [Common Issues](#common-issues)
- [Build Issues](#build-issues)
- [Runtime Issues](#runtime-issues)
- [Configuration Issues](#configuration-issues)
- [Performance Issues](#performance-issues)
- [Data Issues](#data-issues)
- [Network Issues](#network-issues)
- [Debugging Techniques](#debugging-techniques)
- [Getting Help](#getting-help)

## Common Issues

### Service Won't Start

#### Symptoms
- Service fails to start with error messages
- Port already in use errors
- Configuration validation failures

#### Solutions

**Check if ports are available:**
```bash
# Check what's using the ports
netstat -tulpn | grep :8080
netstat -tulpn | grep :9090
netstat -tulpn | grep :4317

# Kill processes using the ports (if safe)
sudo kill -9 <PID>
```

**Validate configuration:**
```bash
# Validate configuration file
cargo run --bin bridge-api -- --config config/bridge.toml --validate

# Check configuration syntax
cargo run --bin bridge-api -- --config config/bridge.toml --check
```

**Check file permissions:**
```bash
# Ensure config files are readable
ls -la config/
chmod 644 config/*.toml

# Check log directory permissions
mkdir -p logs
chmod 755 logs
```

### No Data Received

#### Symptoms
- No telemetry data appears in queries
- Empty metrics endpoints
- No processing statistics

#### Solutions

**Check ingestion endpoints:**
```bash
# Test OTLP HTTP endpoint
curl -X POST http://localhost:4318/v1/metrics \
  -H "Content-Type: application/json" \
  -d '{"resourceMetrics": []}'

# Test gRPC endpoint
grpcurl -plaintext -d '{}' localhost:4317 opentelemetry.proto.collector.metrics.v1.MetricsService/Export
```

**Check service health:**
```bash
# Check API health
curl http://localhost:8080/health

# Check detailed health
curl http://localhost:8080/health/detailed

# Check metrics
curl http://localhost:9090/metrics
```

**Verify data flow:**
```bash
# Check if data is being processed
curl http://localhost:9090/metrics | grep orasi_ingestion

# Check processing pipeline
curl http://localhost:8080/api/v1/status
```

### Query Returns No Results

#### Symptoms
- SQL queries return empty results
- No data in lakehouse tables
- Schema registry shows no schemas

#### Solutions

**Check if data was ingested:**
```bash
# Query raw data
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT COUNT(*) FROM metrics"}'

# Check recent data
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM metrics ORDER BY timestamp DESC LIMIT 10"}'
```

**Verify schema registry:**
```bash
# Check registered schemas
curl http://localhost:8081/schemas

# Check schema compatibility
curl http://localhost:8081/schemas/compatibility
```

**Check lakehouse connectivity:**
```bash
# Test Delta Lake connection
cargo run --example delta_lake_example

# Test other connectors
cargo run --example iceberg_example
cargo run --example snowflake_example
```

## Build Issues

### Compilation Errors

#### Common Rust Compilation Issues

**Missing dependencies:**
```bash
# Update Rust toolchain
rustup update

# Clean and rebuild
cargo clean
cargo build

# Check for outdated dependencies
cargo outdated
cargo update
```

**Feature flag issues:**
```bash
# Build with specific features
cargo build --features "full"

# Check available features
cargo build --help
```

**Platform-specific issues:**
```bash
# Install platform dependencies (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev

# Install platform dependencies (macOS)
brew install openssl pkg-config

# Install platform dependencies (Windows)
# Install Visual Studio Build Tools
```

### Linker Errors

**Missing system libraries:**
```bash
# Ubuntu/Debian
sudo apt-get install -y libssl-dev libpq-dev

# macOS
brew install openssl postgresql

# Windows
# Install vcpkg and required packages
```

**Rust toolchain issues:**
```bash
# Reinstall Rust
rustup self uninstall
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Update components
rustup component add rustfmt clippy
```

## Runtime Issues

### Memory Issues

#### Symptoms
- High memory usage
- Out of memory errors
- Slow performance

#### Solutions

**Monitor memory usage:**
```bash
# Check memory usage
ps aux | grep orasi
top -p $(pgrep -f orasi)

# Monitor with metrics
curl http://localhost:9090/metrics | grep memory
```

**Optimize configuration:**
```toml
# Reduce batch sizes
[ingestion]
batch_size = 500  # Reduce from 1000

# Limit worker threads
[processing]
worker_threads = 2  # Reduce from 4

# Enable garbage collection
[query_engine]
enable_gc = true
gc_interval_seconds = 300
```

**Check for memory leaks:**
```bash
# Run with memory profiling
cargo install flamegraph
cargo flamegraph --bin bridge-api

# Use valgrind (Linux)
valgrind --leak-check=full cargo run --bin bridge-api
```

### Performance Issues

#### Symptoms
- Slow data processing
- High latency
- Low throughput

#### Solutions

**Profile performance:**
```bash
# Run benchmarks
cargo bench

# Profile with perf (Linux)
perf record --call-graph=dwarf cargo run --release --bin bridge-api
perf report

# Profile with instruments (macOS)
cargo install cargo-instruments
cargo instruments --open
```

**Optimize configuration:**
```toml
# Increase worker threads
[processing]
worker_threads = 8  # Increase from 4

# Optimize batch processing
[ingestion]
batch_size = 2000  # Increase from 1000
flush_interval_ms = 1000  # Reduce from 5000

# Enable caching
[query_engine]
enable_query_caching = true
cache_size = 104857600  # 100MB
```

**Check system resources:**
```bash
# Monitor CPU usage
htop

# Monitor disk I/O
iotop

# Monitor network
iftop
```

## Configuration Issues

### Invalid Configuration

#### Symptoms
- Configuration validation errors
- Service fails to start
- Unexpected behavior

#### Solutions

**Validate configuration:**
```bash
# Validate TOML syntax
cargo run --bin bridge-api -- --config config/bridge.toml --validate

# Check for missing fields
cargo run --bin bridge-api -- --config config/bridge.toml --check
```

**Common configuration mistakes:**
```toml
# ❌ Wrong: Missing quotes around strings
host = localhost

# ✅ Correct: Proper string format
host = "localhost"

# ❌ Wrong: Invalid port number
port = 0

# ✅ Correct: Valid port number
port = 8080

# ❌ Wrong: Invalid boolean
enable_feature = yes

# ✅ Correct: Boolean format
enable_feature = true
```

**Environment-specific configuration:**
```bash
# Use environment variables
export ORASI_API_HOST="0.0.0.0"
export ORASI_API_PORT="8080"
cargo run --bin bridge-api

# Use different config files
cargo run --bin bridge-api -- --config config/bridge-dev.toml
```

### Missing Configuration

#### Symptoms
- Service uses default values
- Features not working
- Connection failures

#### Solutions

**Check required configuration:**
```toml
# Required for basic operation
[api]
host = "0.0.0.0"
port = 8080

[grpc]
host = "0.0.0.0"
port = 9090

[ingestion]
otlp_endpoint = "0.0.0.0:4317"
batch_size = 1000
```

**Add optional configuration:**
```toml
# Optional but recommended
[processing]
worker_threads = 4
enable_streaming = true

[query_engine]
worker_threads = 8
enable_query_caching = true

[schema_registry]
host = "0.0.0.0"
port = 8081
```

## Data Issues

### Data Loss

#### Symptoms
- Missing telemetry data
- Incomplete batches
- Data corruption

#### Solutions

**Check data flow:**
```bash
# Monitor ingestion metrics
curl http://localhost:9090/metrics | grep orasi_ingestion_records

# Check processing pipeline
curl http://localhost:8080/api/v1/status/processing

# Verify data persistence
ls -la /tmp/orasi-data/  # or your data directory
```

**Enable data persistence:**
```toml
[ingestion]
enable_persistence = true
persistence_path = "/app/data/persistence"
max_persistence_size = 1073741824  # 1GB
```

**Check error handling:**
```bash
# Check for processing errors
curl http://localhost:9090/metrics | grep orasi_ingestion_errors

# Check dead letter queue
curl http://localhost:8080/api/v1/dead-letter-queue
```

### Data Quality Issues

#### Symptoms
- Invalid data formats
- Missing fields
- Incorrect timestamps

#### Solutions

**Validate data format:**
```bash
# Check data schema
curl http://localhost:8081/schemas

# Validate incoming data
curl -X POST http://localhost:8080/api/v1/validate \
  -H "Content-Type: application/json" \
  -d '{"data": "..."}'
```

**Enable data validation:**
```toml
[ingestion]
enable_validation = true
strict_validation = false  # Allow some flexibility
```

**Check data transformation:**
```bash
# Test data transformation
curl -X POST http://localhost:8080/api/v1/transform \
  -H "Content-Type: application/json" \
  -d '{"data": "...", "transformations": [...]}'
```

## Network Issues

### Connection Problems

#### Symptoms
- Cannot connect to services
- Timeout errors
- Network unreachable

#### Solutions

**Check network connectivity:**
```bash
# Test local connectivity
ping localhost
telnet localhost 8080

# Test external connectivity
ping google.com
curl https://httpbin.org/get
```

**Check firewall settings:**
```bash
# Check if ports are open
sudo ufw status
sudo iptables -L

# Open required ports
sudo ufw allow 8080
sudo ufw allow 9090
sudo ufw allow 4317
```

**Check DNS resolution:**
```bash
# Test DNS resolution
nslookup google.com
dig google.com

# Use IP addresses if DNS fails
# Update configuration to use IP instead of hostname
```

### Load Balancer Issues

#### Symptoms
- Uneven load distribution
- Health check failures
- Connection timeouts

#### Solutions

**Check load balancer configuration:**
```bash
# Test health endpoints
curl http://localhost:8080/health
curl http://localhost:8080/health/live
curl http://localhost:8080/health/ready

# Check load balancer logs
docker logs nginx  # or your load balancer
```

**Configure health checks:**
```toml
[health]
enabled = true
check_interval_seconds = 30
timeout_seconds = 5
failure_threshold = 3
```

## Debugging Techniques

### Logging

**Enable debug logging:**
```bash
# Set log level
export RUST_LOG=debug
cargo run --bin bridge-api

# Set specific log levels
export RUST_LOG=orasi_bridge=debug,orasi_ingestion=info
cargo run --bin bridge-api
```

**Structured logging:**
```bash
# Enable JSON logging
export RUST_LOG_FORMAT=json
cargo run --bin bridge-api

# Log to file
export RUST_LOG_FILE=/tmp/orasi.log
cargo run --bin bridge-api
```

### Profiling

**CPU profiling:**
```bash
# Profile with perf (Linux)
perf record --call-graph=dwarf cargo run --release --bin bridge-api
perf report

# Profile with instruments (macOS)
cargo instruments --open --bin bridge-api
```

**Memory profiling:**
```bash
# Generate flamegraph
cargo flamegraph --bin bridge-api

# Use heim (cross-platform)
cargo install heim
heim memory --pid $(pgrep bridge-api)
```

### Tracing

**Enable distributed tracing:**
```bash
# Set tracing endpoint
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_SERVICE_NAME=orasi-bridge

# Run with tracing
cargo run --bin bridge-api
```

**View traces:**
```bash
# Start Jaeger
docker run -d --name jaeger \
  -e COLLECTOR_OTLP_ENABLED=true \
  -p 16686:16686 \
  -p 4317:4317 \
  jaegertracing/all-in-one:latest

# View traces at http://localhost:16686
```

### Metrics

**Monitor metrics:**
```bash
# View Prometheus metrics
curl http://localhost:9090/metrics

# Start Prometheus
docker run -d --name prometheus \
  -p 9090:9090 \
  -v prometheus.yml:/etc/prometheus/prometheus.yml \
  prom/prometheus

# View metrics at http://localhost:9090
```

## Getting Help

### Before Asking for Help

1. **Check this guide** for your specific issue
2. **Search GitHub Issues** for similar problems
3. **Check the logs** for error messages
4. **Try the debugging techniques** above
5. **Reproduce the issue** in a minimal environment

### When Asking for Help

**Provide the following information:**

1. **Environment details:**
   - Operating system and version
   - Rust version (`rustc --version`)
   - Orasi version
   - Docker version (if applicable)

2. **Error messages:**
   - Full error output
   - Log files
   - Stack traces

3. **Configuration:**
   - Relevant configuration files
   - Environment variables
   - Command line arguments

4. **Steps to reproduce:**
   - Exact commands run
   - Expected vs actual behavior
   - Minimal reproduction case

### Where to Get Help

- **GitHub Issues**: [Report bugs](https://github.com/chytirio/orasi/issues)
- **GitHub Discussions**: [Ask questions](https://github.com/chytirio/orasi/discussions)
- **Documentation**: Check the [docs directory](docs/)
- **Examples**: Look at the [examples directory](examples/)

### Community Resources

- **Stack Overflow**: Tag questions with `orasi` and `rust`
- **Rust Community**: [Rust Forum](https://users.rust-lang.org/)
- **OpenTelemetry**: [OTel Community](https://opentelemetry.io/community/)

---

**Remember**: Most issues can be resolved by checking the logs, validating configuration, and using the debugging techniques described in this guide.
