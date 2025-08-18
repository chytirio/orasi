# Orasi Gateway

API gateway and load balancer for the Orasi OpenTelemetry Data Lake Bridge, providing unified access, routing, and traffic management for the bridge ecosystem.

## Overview

Orasi Gateway serves as the entry point for all client requests to the Orasi bridge system, offering:

- **API Gateway**: Unified API access point with routing and load balancing
- **Load Balancing**: Intelligent traffic distribution across bridge services
- **Rate Limiting**: Request throttling and rate limiting
- **Authentication**: Centralized authentication and authorization
- **Monitoring**: Request/response monitoring and metrics collection
- **Caching**: Response caching for improved performance
- **Circuit Breaker**: Fault tolerance and service protection

## Key Features

### API Gateway
- **Unified Endpoint**: Single entry point for all bridge APIs
- **Request Routing**: Intelligent routing to backend services
- **Protocol Translation**: HTTP/gRPC protocol conversion
- **Request/Response Transformation**: Data format conversion
- **API Versioning**: Support for multiple API versions

### Load Balancing
- **Round Robin**: Simple round-robin load balancing
- **Least Connections**: Route to service with fewest active connections
- **Weighted Round Robin**: Weighted distribution based on service capacity
- **Health-based Routing**: Route only to healthy services
- **Sticky Sessions**: Session affinity for stateful requests

### Traffic Management
- **Rate Limiting**: Per-client and per-endpoint rate limiting
- **Throttling**: Request throttling with configurable policies
- **Circuit Breaker**: Automatic service isolation on failures
- **Retry Logic**: Configurable retry policies with backoff
- **Timeout Management**: Request timeout configuration

### Security
- **Authentication**: JWT token validation and user authentication
- **Authorization**: Role-based access control (RBAC)
- **API Key Management**: API key validation and rate limiting
- **CORS Support**: Cross-origin resource sharing configuration
- **Request Validation**: Input validation and sanitization

### Monitoring & Observability
- **Request Metrics**: Detailed request/response metrics
- **Performance Monitoring**: Latency and throughput monitoring
- **Error Tracking**: Error rate and failure pattern tracking
- **Distributed Tracing**: Request tracing across services
- **Health Monitoring**: Backend service health monitoring

## Quick Start

### Running the Gateway

```bash
# Build the gateway
cargo build --release

# Run with default configuration
./target/release/orasi-gateway

# Run with custom configuration
./target/release/orasi-gateway --config config/gateway.toml

# Run in development mode
cargo run --bin orasi-gateway
```

### Docker Deployment

```bash
# Build Docker image
docker build -t orasi-gateway .

# Run with Docker Compose
docker-compose up -d gateway

# Run standalone container
docker run -p 80:80 -p 443:443 orasi-gateway
```

### Basic Configuration

```toml
# gateway.toml
[gateway]
host = "0.0.0.0"
port = 80
ssl_port = 443
workers = 4

[gateway.ssl]
enabled = true
cert_file = "/etc/ssl/certs/gateway.crt"
key_file = "/etc/ssl/private/gateway.key"

[gateway.routing]
default_service = "bridge-api"
enable_health_checks = true
health_check_interval = 30

[gateway.load_balancing]
algorithm = "round_robin"
enable_sticky_sessions = false
session_timeout = 3600

[gateway.rate_limiting]
enabled = true
requests_per_minute = 1000
burst_size = 100

[gateway.circuit_breaker]
enabled = true
failure_threshold = 5
recovery_timeout = 60
half_open_max_requests = 3

[gateway.caching]
enabled = true
cache_size_mb = 512
ttl_seconds = 300

[gateway.auth]
enabled = true
jwt_secret = "your-secret-key"
api_key_header = "X-API-Key"

[gateway.monitoring]
metrics_enabled = true
metrics_port = 9090
tracing_enabled = true
```

## API Routing

### Service Discovery

The gateway automatically discovers and routes to backend services:

```toml
[gateway.services]
bridge-api = "http://bridge-api:8080"
query-engine = "http://query-engine:8081"
schema-registry = "http://schema-registry:8082"
streaming-processor = "http://streaming-processor:8083"
```

### Route Configuration

```toml
[gateway.routes]
# API routes
"/v1/ingest/*" = { service = "bridge-api", rate_limit = 1000 }
"/v1/query/*" = { service = "query-engine", rate_limit = 500 }
"/v1/schemas/*" = { service = "schema-registry", rate_limit = 200 }
"/v1/stream/*" = { service = "streaming-processor", rate_limit = 100 }

# Health and monitoring routes
"/health" = { service = "bridge-api", cache = false }
"/metrics" = { service = "bridge-api", cache = false }
"/ready" = { service = "bridge-api", cache = false }
```

### Load Balancing Configuration

```toml
[gateway.load_balancing]
algorithm = "least_connections"
health_check_path = "/health"
health_check_interval = 30
health_check_timeout = 5

[gateway.load_balancing.services.bridge-api]
instances = [
    "http://bridge-api-1:8080",
    "http://bridge-api-2:8080",
    "http://bridge-api-3:8080"
]
weights = [1, 1, 1]
```

## Usage Examples

### Basic API Request

```bash
# Request through gateway
curl -X POST http://gateway:80/v1/ingest/otlp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-jwt-token" \
  -d '{
    "resource": {
      "attributes": {
        "service.name": "my-service"
      }
    },
    "spans": [
      {
        "name": "http_request",
        "start_time": "2024-01-01T00:00:00Z",
        "end_time": "2024-01-01T00:00:01Z"
      }
    ]
  }'
```

### gRPC Request

```rust
use tonic::{transport::Channel, Request};
use orasi_gateway::grpc::{
    gateway_service_client::GatewayServiceClient,
    IngestRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to gateway
    let channel = Channel::from_static("http://gateway:80")
        .connect()
        .await?;
    
    let mut client = GatewayServiceClient::new(channel);
    
    // Request will be routed to appropriate backend service
    let request = Request::new(IngestRequest {
        data: "telemetry data".to_string(),
    });
    
    let response = client.ingest(request).await?;
    println!("Response: {:?}", response);
    
    Ok(())
}
```

### Rate Limiting Example

```bash
# Request with rate limiting
for i in {1..10}; do
  curl -X GET http://gateway:80/v1/schemas \
    -H "X-API-Key: your-api-key"
  echo "Request $i"
  sleep 0.1
done
```

## Architecture

The Orasi Gateway follows a layered architecture with clear separation of concerns:

```
┌─────────────────┐
│   Orasi Gateway │
├─────────────────┤
│  Request Router │
│  Load Balancer  │
│  Rate Limiter   │
│  Auth Middleware│
│  Cache Layer    │
│  Circuit Breaker│
├─────────────────┤
│  Service Discovery│
│  Health Monitor │
│  Metrics        │
│  Tracing        │
└─────────────────┘
```

### Core Components

1. **Request Router**: Routes requests to appropriate backend services
2. **Load Balancer**: Distributes traffic across service instances
3. **Rate Limiter**: Enforces rate limiting policies
4. **Auth Middleware**: Handles authentication and authorization
5. **Cache Layer**: Caches responses for improved performance
6. **Circuit Breaker**: Protects against cascading failures
7. **Service Discovery**: Discovers and monitors backend services
8. **Health Monitor**: Monitors backend service health
9. **Metrics**: Collects and exposes metrics
10. **Tracing**: Distributed tracing support

## Configuration Reference

### Gateway Configuration

```toml
[gateway]
# Basic settings
host = "0.0.0.0"
port = 80
ssl_port = 443
workers = 4

# SSL/TLS configuration
[gateway.ssl]
enabled = true
cert_file = "/etc/ssl/certs/gateway.crt"
key_file = "/etc/ssl/private/gateway.key"
ca_file = "/etc/ssl/certs/ca.crt"

# Routing configuration
[gateway.routing]
default_service = "bridge-api"
enable_health_checks = true
health_check_interval = 30
health_check_timeout = 5

# Load balancing configuration
[gateway.load_balancing]
algorithm = "round_robin"  # round_robin, least_connections, weighted
enable_sticky_sessions = false
session_timeout = 3600

# Rate limiting configuration
[gateway.rate_limiting]
enabled = true
requests_per_minute = 1000
burst_size = 100
per_client = true
per_endpoint = true

# Circuit breaker configuration
[gateway.circuit_breaker]
enabled = true
failure_threshold = 5
recovery_timeout = 60
half_open_max_requests = 3

# Caching configuration
[gateway.caching]
enabled = true
cache_size_mb = 512
ttl_seconds = 300
max_entries = 10000

# Authentication configuration
[gateway.auth]
enabled = true
jwt_secret = "your-secret-key"
api_key_header = "X-API-Key"
token_expiry_seconds = 3600

# Monitoring configuration
[gateway.monitoring]
metrics_enabled = true
metrics_port = 9090
tracing_enabled = true
log_level = "info"
```

### Service Configuration

```toml
[gateway.services]
bridge-api = "http://bridge-api:8080"
query-engine = "http://query-engine:8081"
schema-registry = "http://schema-registry:8082"
streaming-processor = "http://streaming-processor:8083"

[gateway.services.bridge-api]
instances = [
    "http://bridge-api-1:8080",
    "http://bridge-api-2:8080",
    "http://bridge-api-3:8080"
]
weights = [1, 1, 1]
health_check_path = "/health"
```

### Route Configuration

```toml
[gateway.routes]
# API routes with specific configurations
"/v1/ingest/*" = { 
    service = "bridge-api", 
    rate_limit = 1000,
    cache = false,
    timeout = 30
}

"/v1/query/*" = { 
    service = "query-engine", 
    rate_limit = 500,
    cache = true,
    timeout = 60
}

"/v1/schemas/*" = { 
    service = "schema-registry", 
    rate_limit = 200,
    cache = true,
    timeout = 10
}

# Health and monitoring routes
"/health" = { 
    service = "bridge-api", 
    cache = false,
    auth = false
}

"/metrics" = { 
    service = "bridge-api", 
    cache = false,
    auth = false
}
```

## Development

### Building

```bash
# Build the gateway
cargo build

# Build with optimizations
cargo build --release

# Build specific binary
cargo build --bin orasi-gateway

# Build with features
cargo build --features ssl
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_routing

# Run integration tests
cargo test --test integration

# Run with logging
RUST_LOG=debug cargo test
```

### Development Server

```bash
# Run development server
cargo run --bin orasi-gateway

# Run with hot reload
cargo watch -x run --bin orasi-gateway

# Run with custom config
cargo run --bin orasi-gateway -- --config config/dev.toml
```

## Deployment

### Docker Deployment

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin orasi-gateway

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/orasi-gateway /usr/local/bin/
EXPOSE 80 443 9090
CMD ["orasi-gateway"]
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: orasi-gateway
spec:
  replicas: 3
  selector:
    matchLabels:
      app: orasi-gateway
  template:
    metadata:
      labels:
        app: orasi-gateway
    spec:
      containers:
      - name: gateway
        image: orasi/gateway:latest
        ports:
        - containerPort: 80
        - containerPort: 443
        - containerPort: 9090
        env:
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
---
apiVersion: v1
kind: Service
metadata:
  name: orasi-gateway
spec:
  selector:
    app: orasi-gateway
  ports:
  - name: http
    port: 80
    targetPort: 80
  - name: https
    port: 443
    targetPort: 443
  - name: metrics
    port: 9090
    targetPort: 9090
  type: LoadBalancer
```

## Monitoring

### Metrics

The gateway exposes Prometheus metrics at `/metrics`:

- **Request metrics**: Request count, duration, error rates
- **Load balancing metrics**: Service health, connection counts
- **Rate limiting metrics**: Rate limit hits and throttling
- **Circuit breaker metrics**: Failure counts and state changes
- **Cache metrics**: Hit rates and cache size
- **System metrics**: CPU, memory, and network usage

### Health Checks

Health check endpoints:

- `/health`: Overall gateway health
- `/ready`: Readiness for traffic
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
