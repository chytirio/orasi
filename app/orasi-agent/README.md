# Orasi Agent

Data collection and processing agent for the Orasi OpenTelemetry Data Lake Bridge, providing distributed telemetry data collection, processing, and forwarding capabilities.

## Overview

Orasi Agent is a lightweight, distributed agent that collects, processes, and forwards OpenTelemetry telemetry data to the bridge system, offering:

- **Data Collection**: Multi-source telemetry data collection
- **Local Processing**: On-agent data processing and filtering
- **Buffering & Batching**: Efficient data buffering and batch transmission
- **Reliability**: Fault-tolerant data transmission with retry logic
- **Resource Management**: Low resource footprint and efficient memory usage
- **Configuration Management**: Dynamic configuration updates
- **Health Monitoring**: Built-in health monitoring and metrics

## Key Features

### Data Collection
- **Multiple Sources**: Log files, system metrics, application metrics
- **Protocol Support**: OTLP, Prometheus, StatsD, custom protocols
- **File Monitoring**: Real-time log file monitoring and parsing
- **System Metrics**: CPU, memory, disk, network metrics collection
- **Application Metrics**: Custom application metric collection
- **Event Streaming**: Real-time event streaming and processing

### Data Processing
- **Local Filtering**: Configurable data filtering and routing
- **Data Transformation**: Format conversion and enrichment
- **Aggregation**: Time-window based data aggregation
- **Sampling**: Intelligent data sampling and rate limiting
- **Compression**: Data compression for efficient transmission
- **Validation**: Data validation and quality checks

### Reliability & Performance
- **Buffering**: In-memory and disk-based buffering
- **Batching**: Configurable batch sizes and timeouts
- **Retry Logic**: Exponential backoff retry mechanisms
- **Circuit Breaker**: Automatic failure detection and recovery
- **Load Balancing**: Multiple endpoint load balancing
- **Compression**: Data compression for bandwidth optimization

### Resource Management
- **Memory Management**: Efficient memory usage and garbage collection
- **CPU Optimization**: Configurable CPU usage limits
- **Disk Usage**: Configurable disk buffer limits
- **Network Optimization**: Bandwidth usage optimization
- **Process Management**: Graceful shutdown and restart

## Quick Start

### Running the Agent

```bash
# Build the agent
cargo build --release

# Run with default configuration
./target/release/orasi-agent

# Run with custom configuration
./target/release/orasi-agent --config config/agent.toml

# Run in development mode
cargo run --bin orasi-agent
```

### Docker Deployment

```bash
# Build Docker image
docker build -t orasi-agent .

# Run with Docker Compose
docker-compose up -d agent

# Run standalone container
docker run -v /var/log:/var/log:ro orasi-agent
```

### Basic Configuration

```toml
# agent.toml
[agent]
name = "my-agent"
version = "0.1.0"
environment = "production"

[agent.collection]
enabled = true
interval_seconds = 30
max_concurrent_collectors = 4

[agent.processing]
enabled = true
batch_size = 1000
batch_timeout_ms = 5000
max_memory_mb = 512

[agent.transmission]
endpoints = ["http://bridge-api:8080/v1/ingest"]
retry_attempts = 3
retry_delay_ms = 1000
timeout_seconds = 30

[agent.monitoring]
metrics_enabled = true
metrics_port = 9090
health_check_enabled = true
health_check_port = 8081

[agent.logging]
level = "info"
format = "json"
file_path = "/var/log/orasi-agent.log"
```

## Data Collection Configuration

### Log File Collection

```toml
[agent.sources.logs]
enabled = true
paths = [
    "/var/log/application/*.log",
    "/var/log/system/*.log"
]
patterns = ["*.log", "*.log.*"]
exclude_patterns = ["*.log.gz", "*.log.old"]

[agent.sources.logs.parsing]
format = "json"  # json, csv, regex, custom
timestamp_field = "timestamp"
timestamp_format = "%Y-%m-%d %H:%M:%S"

[agent.sources.logs.fields]
service_name = "my-service"
environment = "production"
```

### System Metrics Collection

```toml
[agent.sources.system]
enabled = true
interval_seconds = 60

[agent.sources.system.metrics]
cpu = true
memory = true
disk = true
network = true
processes = true

[agent.sources.system.filters]
min_cpu_usage = 1.0
min_memory_usage = 1.0
```

### Application Metrics Collection

```toml
[agent.sources.application]
enabled = true
endpoint = "http://localhost:8080/metrics"
interval_seconds = 30

[agent.sources.application.metrics]
http_requests = true
database_queries = true
cache_hits = true
error_rates = true

[agent.sources.application.labels]
service_name = "my-service"
version = "1.0.0"
```

## Data Processing Configuration

### Filtering Rules

```toml
[agent.processing.filters]
enabled = true

[agent.processing.filters.rules]
# Filter out debug logs
"log_level" = { operator = "!=", value = "debug" }

# Filter high CPU usage
"cpu_usage" = { operator = ">", value = 80.0 }

# Filter specific services
"service_name" = { operator = "in", value = ["web", "api", "db"] }
```

### Transformation Rules

```toml
[agent.processing.transformations]
enabled = true

[agent.processing.transformations.rules]
# Add timestamp
"timestamp" = { type = "timestamp", format = "unix" }

# Rename fields
"cpu_percent" = { type = "rename", to = "cpu_usage" }

# Convert types
"memory_bytes" = { type = "convert", to = "float" }

# Add labels
"environment" = { type = "constant", value = "production" }
```

### Aggregation Rules

```toml
[agent.processing.aggregations]
enabled = true
window_seconds = 60

[agent.processing.aggregations.rules]
# CPU usage average
"cpu_usage" = { type = "average", window = 60 }

# Request count sum
"request_count" = { type = "sum", window = 60 }

# Error rate percentage
"error_rate" = { type = "percentage", window = 60 }
```

## Usage Examples

### Basic Agent Usage

```bash
# Start agent with configuration
./orasi-agent --config config/agent.toml

# Start with environment variables
ORASI_AGENT_NAME=my-agent \
ORASI_BRIDGE_ENDPOINT=http://bridge-api:8080 \
./orasi-agent

# Start with log file monitoring
./orasi-agent --log-paths /var/log/app/*.log
```

### Configuration Management

```bash
# Reload configuration
curl -X POST http://localhost:8081/config/reload

# Get current configuration
curl http://localhost:8081/config

# Update configuration
curl -X PUT http://localhost:8081/config \
  -H "Content-Type: application/json" \
  -d '{"collection": {"interval_seconds": 60}}'
```

### Health Monitoring

```bash
# Check agent health
curl http://localhost:8081/health

# Get agent metrics
curl http://localhost:9090/metrics

# Get agent status
curl http://localhost:8081/status
```

## Architecture

The Orasi Agent follows a modular architecture with clear separation of concerns:

```
┌─────────────────┐
│   Orasi Agent   │
├─────────────────┤
│  Data Collectors│
│  Processors     │
│  Buffers        │
│  Transmitters   │
├─────────────────┤
│  Configuration  │
│  Health Monitor │
│  Metrics        │
│  Logging        │
└─────────────────┘
```

### Core Components

1. **Data Collectors**: Collect data from various sources
2. **Processors**: Process, filter, and transform data
3. **Buffers**: Buffer data for efficient transmission
4. **Transmitters**: Transmit data to bridge endpoints
5. **Configuration**: Dynamic configuration management
6. **Health Monitor**: Health monitoring and status reporting
7. **Metrics**: Agent metrics collection and export
8. **Logging**: Structured logging and error reporting

## Configuration Reference

### Agent Configuration

```toml
[agent]
# Basic settings
name = "my-agent"
version = "0.1.0"
environment = "production"
data_dir = "/var/lib/orasi-agent"

# Collection settings
[agent.collection]
enabled = true
interval_seconds = 30
max_concurrent_collectors = 4
buffer_size = 10000

# Processing settings
[agent.processing]
enabled = true
batch_size = 1000
batch_timeout_ms = 5000
max_memory_mb = 512
enable_compression = true

# Transmission settings
[agent.transmission]
endpoints = ["http://bridge-api:8080/v1/ingest"]
retry_attempts = 3
retry_delay_ms = 1000
timeout_seconds = 30
enable_compression = true

# Monitoring settings
[agent.monitoring]
metrics_enabled = true
metrics_port = 9090
health_check_enabled = true
health_check_port = 8081
log_level = "info"
```

### Source Configuration

```toml
# Log file sources
[agent.sources.logs]
enabled = true
paths = ["/var/log/*.log"]
patterns = ["*.log"]
exclude_patterns = ["*.log.gz"]

[agent.sources.logs.parsing]
format = "json"
timestamp_field = "timestamp"
timestamp_format = "%Y-%m-%d %H:%M:%S"

# System metrics sources
[agent.sources.system]
enabled = true
interval_seconds = 60

[agent.sources.system.metrics]
cpu = true
memory = true
disk = true
network = true

# Application metrics sources
[agent.sources.application]
enabled = true
endpoint = "http://localhost:8080/metrics"
interval_seconds = 30

[agent.sources.application.metrics]
http_requests = true
database_queries = true
```

### Processing Configuration

```toml
# Filtering configuration
[agent.processing.filters]
enabled = true

[agent.processing.filters.rules]
"log_level" = { operator = "!=", value = "debug" }
"cpu_usage" = { operator = ">", value = 80.0 }

# Transformation configuration
[agent.processing.transformations]
enabled = true

[agent.processing.transformations.rules]
"timestamp" = { type = "timestamp", format = "unix" }
"cpu_percent" = { type = "rename", to = "cpu_usage" }

# Aggregation configuration
[agent.processing.aggregations]
enabled = true
window_seconds = 60

[agent.processing.aggregations.rules]
"cpu_usage" = { type = "average", window = 60 }
"request_count" = { type = "sum", window = 60 }
```

## Development

### Building

```bash
# Build the agent
cargo build

# Build with optimizations
cargo build --release

# Build specific binary
cargo build --bin orasi-agent

# Build with features
cargo build --features system-metrics
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_collection

# Run integration tests
cargo test --test integration

# Run with logging
RUST_LOG=debug cargo test
```

### Development Server

```bash
# Run development server
cargo run --bin orasi-agent

# Run with hot reload
cargo watch -x run --bin orasi-agent

# Run with custom config
cargo run --bin orasi-agent -- --config config/dev.toml
```

## Deployment

### Docker Deployment

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin orasi-agent

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/orasi-agent /usr/local/bin/
EXPOSE 8081 9090
CMD ["orasi-agent"]
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: orasi-agent
spec:
  selector:
    matchLabels:
      app: orasi-agent
  template:
    metadata:
      labels:
        app: orasi-agent
    spec:
      containers:
      - name: agent
        image: orasi/agent:latest
        ports:
        - containerPort: 8081
        - containerPort: 9090
        env:
        - name: RUST_LOG
          value: "info"
        volumeMounts:
        - name: logs
          mountPath: /var/log
          readOnly: true
        - name: config
          mountPath: /etc/orasi-agent
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
      volumes:
      - name: logs
        hostPath:
          path: /var/log
      - name: config
        configMap:
          name: orasi-agent-config
```

## Monitoring

### Metrics

The agent exposes Prometheus metrics at `/metrics`:

- **Collection metrics**: Data collection rates and volumes
- **Processing metrics**: Processing latency and throughput
- **Transmission metrics**: Transmission success rates and latency
- **Buffer metrics**: Buffer usage and overflow rates
- **System metrics**: CPU, memory, and disk usage

### Health Checks

Health check endpoints:

- `/health`: Overall agent health
- `/ready`: Readiness for data collection
- `/live`: Liveness check

### Logging

Structured logging with configurable levels:

```bash
# Set log level
export RUST_LOG=info

# Enable JSON logging
export RUST_LOG_JSON=true
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
