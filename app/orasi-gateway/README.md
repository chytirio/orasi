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

# Run the example
cargo run --example basic_gateway_example
```

### Docker Deployment

```bash
# Build Docker image
docker build -t orasi-gateway .

# Run with Docker Compose
docker-compose up -d

# Run standalone container
docker run -p 8080:8080 -p 8081:8081 orasi-gateway
```

### Environment Variables

```bash
# Gateway configuration
export GATEWAY_ID="orasi-gateway-1"
export GATEWAY_ENDPOINT="0.0.0.0:8080"
export HEALTH_ENDPOINT="0.0.0.0:8081"
export METRICS_ENDPOINT="0.0.0.0:9090"
export ADMIN_ENDPOINT="0.0.0.0:8082"

# Logging
export RUST_LOG="info"

# Configuration file
export GATEWAY_CONFIG="config/gateway.toml"
```

## Configuration

The gateway supports comprehensive configuration through TOML files:

```toml
# Gateway identification
gateway_id = "orasi-gateway-1"

# Gateway endpoints
gateway_endpoint = "0.0.0.0:8080"
health_endpoint = "0.0.0.0:8081"
metrics_endpoint = "0.0.0.0:9090"
admin_endpoint = "0.0.0.0:8082"

# Service discovery configuration
[service_discovery]
backend = "Etcd"
endpoints = ["http://localhost:2379"]
refresh_interval = 30
health_check_interval = 10

# Load balancing configuration
[load_balancing]
algorithm = "RoundRobin"
health_check_enabled = true
health_check_interval = 30
health_check_timeout = 5
sticky_sessions_enabled = false
sticky_session_timeout = 300

# Routing configuration
[routing]
timeout_seconds = 30
max_retries = 3
retry_delay_ms = 1000
circuit_breaker_enabled = true
circuit_breaker_threshold = 5
circuit_breaker_timeout = 60

# Default routes
[[routing.routes]]
path = "/api/v1/*"
method = "*"
service_name = "bridge-api"
priority = 100
rules = []

[[routing.routes]]
path = "/agent/*"
method = "*"
service_name = "orasi-agent"
priority = 90
rules = []

# Rate limiting configuration
[rate_limiting]
global_rps = 1000
per_client_rps = 100
per_endpoint_rps = 500
burst_size = 200
window_size_seconds = 60
enabled = true
```

## HTTP API

The gateway exposes a comprehensive HTTP API for management and monitoring:

### Health Endpoints

```bash
# Overall health check
GET /health

# Liveness probe
GET /health/live

# Readiness probe
GET /health/ready
```

### Metrics Endpoints

```bash
# JSON metrics
GET /metrics

# Prometheus metrics
GET /metrics/prometheus
```

### Gateway Management

```bash
# Gateway information
GET /gateway/info

# Gateway status
GET /gateway/status

# List all routes
GET /gateway/routes

# Add new route
POST /gateway/routes
Content-Type: application/json

{
  "path": "/api/v2/*",
  "method": "*",
  "service_name": "bridge-api-v2",
  "priority": 95,
  "rules": []
}

# Remove route
DELETE /gateway/routes/api/v2/*?method=GET
```

### Load Balancer Management

```bash
# List all endpoints
GET /loadbalancer/endpoints

# Get service endpoints
GET /loadbalancer/endpoints/bridge-api

# Load balancer health
GET /loadbalancer/health
```

### Rate Limiter Management

```bash
# Get rate limit statistics
GET /ratelimiter/stats

# Reset rate limit counters
POST /ratelimiter/reset
```

### Example API Usage

```bash
# Check gateway health
curl http://localhost:8081/health

# Get gateway metrics
curl http://localhost:9090/metrics/prometheus

# List all routes
curl http://localhost:8082/gateway/routes

# Add a new route
curl -X POST http://localhost:8082/gateway/routes \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/api/v1/users/*",
    "method": "*",
    "service_name": "user-service",
    "priority": 80,
    "rules": []
  }'

# Get rate limit statistics
curl http://localhost:8082/ratelimiter/stats
```

## Load Balancing

The gateway supports multiple load balancing algorithms:

### Round Robin
```toml
[load_balancing]
algorithm = "RoundRobin"
```

### Least Connections
```toml
[load_balancing]
algorithm = "LeastConnections"
```

### Weighted Round Robin
```toml
[load_balancing]
algorithm = "WeightedRoundRobin"
```

### Random Selection
```toml
[load_balancing]
algorithm = "Random"
```

### IP Hash
```toml
[load_balancing]
algorithm = "IpHash"
```

## Rate Limiting

The gateway implements comprehensive rate limiting:

### Global Rate Limiting
```toml
[rate_limiting]
global_rps = 1000
```

### Per-client Rate Limiting
```toml
[rate_limiting]
per_client_rps = 100
```

### Per-endpoint Rate Limiting
```toml
[rate_limiting]
per_endpoint_rps = 500
```

### Rate Limit Headers
The gateway adds rate limit headers to responses:
- `X-RateLimit-Limit`: Request limit per window
- `X-RateLimit-Remaining`: Remaining requests in current window
- `X-RateLimit-Reset`: Time when the rate limit resets
- `Retry-After`: Time to wait before retrying (when rate limited)

## Routing

The gateway supports flexible routing with multiple matching criteria:

### Path-based Routing
```toml
[[routing.routes]]
path = "/api/v1/*"
method = "*"
service_name = "bridge-api"
```

### Method-based Routing
```toml
[[routing.routes]]
path = "/api/v1/users"
method = "GET"
service_name = "user-service"
```

### Header-based Routing
```toml
[[routing.routes]]
path = "/api/v1/data"
method = "POST"
service_name = "data-service"
rules = [
  { type = "header", name = "X-Version", value = "v2" }
]
```

### Query Parameter Routing
```toml
[[routing.routes]]
path = "/api/v1/search"
method = "GET"
service_name = "search-service"
rules = [
  { type = "query_param", name = "engine", value = "elasticsearch" }
]
```

### Route Priority
Routes are matched by priority (higher numbers = higher priority):
```toml
[[routing.routes]]
path = "/api/v1/health"
method = "GET"
service_name = "health-service"
priority = 200  # High priority
```

## Monitoring

### Prometheus Metrics

The gateway exports comprehensive Prometheus metrics:

```bash
# Gateway metrics
orasi_gateway_up 1
orasi_gateway_requests_total{method="GET",path="/api/v1/health",status="200"} 150
orasi_gateway_request_duration_seconds{method="GET",path="/api/v1/health"} 0.05
orasi_gateway_rate_limited_requests_total{client="client-1"} 5
```

### Health Checks

The gateway provides multiple health check endpoints:

```bash
# Overall health
curl http://localhost:8081/health

# Liveness probe (for Kubernetes)
curl http://localhost:8081/health/live

# Readiness probe (for Kubernetes)
curl http://localhost:8081/health/ready
```

### Logging

The gateway uses structured logging with configurable levels:

```bash
# Set log level
export RUST_LOG="info"

# Enable debug logging
export RUST_LOG="debug"
```

## Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/chytirio/orasi.git
cd orasi

# Build the gateway
cargo build --release --bin orasi-gateway

# Run tests
cargo test --bin orasi-gateway

# Run the example
cargo run --example basic_gateway_example
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_routing

# Run with logging
RUST_LOG=debug cargo test
```

### Code Structure

```
src/
├── main.rs              # Binary entry point
├── lib.rs               # Library entry point
├── config.rs            # Configuration management
├── error.rs             # Error types
├── types.rs             # Type definitions
├── gateway/             # Core gateway implementation
│   ├── core.rs          # Main gateway orchestrator
│   ├── health.rs        # Health checking
│   ├── rate_limiter.rs  # Rate limiting
│   └── state.rs         # State management
├── routing/             # Routing system
│   ├── core.rs          # Router implementation
│   ├── matcher.rs       # Route matching
│   └── proxy.rs         # HTTP proxy
├── load_balancer.rs     # Load balancing
├── discovery.rs         # Service discovery
├── metrics.rs           # Metrics collection
└── http.rs              # HTTP server
```

## Deployment

### Docker

```bash
# Build image
docker build -t orasi-gateway .

# Run container
docker run -d \
  --name orasi-gateway \
  -p 8080:8080 \
  -p 8081:8081 \
  -p 9090:9090 \
  -p 8082:8082 \
  -v $(pwd)/config:/app/config \
  orasi-gateway
```

### Docker Compose

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f orasi-gateway

# Stop services
docker-compose down
```

### Kubernetes

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
      - name: orasi-gateway
        image: orasi-gateway:latest
        ports:
        - containerPort: 8080
        - containerPort: 8081
        - containerPort: 9090
        - containerPort: 8082
        env:
        - name: RUST_LOG
          value: "info"
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8081
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8081
          initialDelaySeconds: 5
          periodSeconds: 5
```

## Troubleshooting

### Common Issues

1. **Gateway won't start**
   - Check configuration file syntax
   - Verify all required environment variables
   - Check port availability

2. **Routes not working**
   - Verify route configuration
   - Check backend service availability
   - Review gateway logs

3. **Rate limiting too aggressive**
   - Adjust rate limit configuration
   - Check client identification
   - Review rate limit statistics

4. **High latency**
   - Check backend service health
   - Review load balancer configuration
   - Monitor resource usage

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG="debug"

# Run with debug output
cargo run --bin orasi-gateway
```

### Health Checks

```bash
# Check gateway health
curl http://localhost:8081/health

# Check specific component health
curl http://localhost:8082/loadbalancer/health
curl http://localhost:8082/ratelimiter/stats
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run the test suite
6. Submit a pull request

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](../../LICENSE) file for details.
