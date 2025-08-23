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
- **Cluster Coordination**: Distributed coordination and service discovery
- **HTTP API**: Comprehensive REST API for monitoring and management

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

### Cluster Coordination
- **Service Discovery**: Integration with etcd and Consul
- **Leader Election**: Distributed leadership management
- **Cluster Membership**: Dynamic agent registration and health monitoring
- **Load Balancing**: Multiple endpoint load balancing
- **State Synchronization**: Cluster state management

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

# Run with Docker Compose (includes etcd, Prometheus, Grafana)
docker-compose up -d

# Run standalone container
docker run -v /var/log:/var/log:ro orasi-agent
```

### Environment Variables

```bash
export ORASI_AGENT_ID="agent-001"
export ORASI_AGENT_ENDPOINT="0.0.0.0:8082"
export ORASI_HEALTH_ENDPOINT="0.0.0.0:8083"
export ORASI_METRICS_ENDPOINT="0.0.0.0:9092"
export ORASI_CLUSTER_ENDPOINT="0.0.0.0:8084"
export RUST_LOG="info"
```

## Configuration

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

[cluster]
service_discovery = "Etcd"
service_discovery_endpoints = ["http://localhost:2379"]
heartbeat_interval = 30
session_timeout = 90

[cluster.leader_election]
enabled = true
election_timeout = 10

[capabilities]
max_concurrent_tasks = 10
supported_formats = ["json", "parquet", "avro", "csv"]

[processing]
max_concurrent_tasks = 10
batch_size = 1000
batch_timeout_ms = 5000
max_memory_mb = 512

[storage]
data_directory = "/var/lib/orasi/agent"
temp_directory = "/tmp/orasi/agent"
max_disk_usage_gb = 10

[health]
check_interval = 30
resource_thresholds = { cpu_percent = 80.0, memory_percent = 80.0, disk_percent = 80.0 }

[metrics]
collection_interval = 15
prometheus_enabled = true
custom_metrics_enabled = true
```

## HTTP API

The agent exposes a comprehensive HTTP API for monitoring and management:

### Health Endpoints

- `GET /health` - Comprehensive health check
- `GET /health/live` - Liveness probe
- `GET /health/ready` - Readiness probe

### Metrics Endpoints

- `GET /metrics` - JSON metrics
- `GET /metrics/prometheus` - Prometheus metrics

### Agent Management

- `GET /agent/info` - Agent information
- `GET /agent/status` - Agent status
- `GET /agent/tasks` - Task queue statistics
- `POST /agent/tasks` - Submit new task

### Cluster Management

- `GET /cluster/state` - Cluster state
- `GET /cluster/members` - Cluster members
- `GET /cluster/leader` - Leader status

### Example API Usage

```bash
# Health check
curl http://localhost:8083/health

# Get agent info
curl http://localhost:8082/agent/info

# Get metrics
curl http://localhost:9092/metrics

# Submit task
curl -X POST http://localhost:8082/agent/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "task_id": "task-001",
    "task_type": "Ingestion",
    "priority": "Normal",
    "payload": {
      "Ingestion": {
        "source_id": "example-source",
        "format": "json",
        "location": "file:///tmp/data.json"
      }
    }
  }'
```

## Task Processing

The agent supports various types of tasks:

### Task Types

- **Ingestion**: Data ingestion from various sources
- **Indexing**: Building and maintaining indexes
- **Processing**: Data transformation and analysis
- **Query**: Query execution
- **Maintenance**: System maintenance tasks

### Task Priority Levels

- **Low**: Background tasks
- **Normal**: Standard processing tasks
- **High**: Important tasks
- **Critical**: Urgent tasks

### Task Lifecycle

1. **Submitted**: Task is submitted to the queue
2. **Pending**: Task is waiting in the priority queue
3. **Running**: Task is being processed
4. **Completed**: Task completed successfully
5. **Failed**: Task failed (with retry logic)
6. **Retrying**: Task is being retried after failure

## Monitoring

### Health Checks

The agent provides comprehensive health monitoring:

- **Resource Usage**: CPU, memory, disk usage
- **Agent State**: Running, stopped, error states
- **Connectivity**: Network connectivity checks
- **Task Processing**: Queue health and processing status

### Metrics

The agent exposes detailed metrics:

- **Task Metrics**: Processing rates, success/failure rates
- **Resource Metrics**: CPU, memory, disk usage
- **Performance Metrics**: Response times, throughput
- **Error Metrics**: Error rates and types

### Prometheus Integration

The agent exports Prometheus metrics for integration with monitoring systems:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'orasi-agent'
    static_configs:
      - targets: ['localhost:9092']
    metrics_path: '/metrics/prometheus'
    scrape_interval: 15s
```

## Cluster Coordination

### Service Discovery

The agent supports multiple service discovery backends:

- **etcd**: Distributed key-value store
- **Consul**: Service mesh and discovery
- **Static**: Static configuration

### Leader Election

The agent implements distributed leader election for coordination:

- **Election Timeout**: Configurable election timeouts
- **Leader Responsibilities**: Task distribution, state management
- **Failover**: Automatic leader failover

### Cluster Membership

- **Dynamic Registration**: Automatic agent registration
- **Health Monitoring**: Member health tracking
- **Load Balancing**: Distributed task processing

## Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/chytirio/orasi.git
cd orasi/app/orasi-agent

# Build the agent
cargo build --release

# Run tests
cargo test

# Run example
cargo run --example basic_agent_example
```

### Development Setup

```bash
# Install dependencies
cargo install cargo-watch

# Run with hot reload
cargo watch -x run --bin orasi-agent

# Run with specific log level
RUST_LOG=debug cargo run --bin orasi-agent
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_agent_creation

# Run integration tests
cargo test --test integration
```

## Deployment

### Docker Compose

The included `docker-compose.yml` provides a complete development environment:

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f orasi-agent-1

# Stop services
docker-compose down
```

### Kubernetes

Example Kubernetes deployment:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: orasi-agent
spec:
  replicas: 3
  selector:
    matchLabels:
      app: orasi-agent
  template:
    metadata:
      labels:
        app: orasi-agent
    spec:
      containers:
      - name: orasi-agent
        image: orasi-agent:latest
        ports:
        - containerPort: 8082
        - containerPort: 8083
        - containerPort: 9092
        env:
        - name: ORASI_AGENT_ID
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: RUST_LOG
          value: "info"
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8083
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8083
          initialDelaySeconds: 5
          periodSeconds: 5
```

## Troubleshooting

### Common Issues

1. **Agent won't start**: Check configuration and dependencies
2. **Tasks not processing**: Verify task queue and processor status
3. **Health checks failing**: Check resource usage and connectivity
4. **Cluster issues**: Verify service discovery configuration

### Debugging

```bash
# Enable debug logging
export RUST_LOG=debug

# Check agent status
curl http://localhost:8082/agent/status

# Check health
curl http://localhost:8083/health

# Check metrics
curl http://localhost:9092/metrics
```

### Logs

The agent provides comprehensive logging:

- **Application Logs**: Agent operation logs
- **Task Logs**: Task processing logs
- **Health Logs**: Health check results
- **Cluster Logs**: Cluster coordination logs

## Contributing

Please read [CONTRIBUTING.md](../../CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](../../LICENSE) file for details.
