# Orasi Gateway Metrics

This document describes the metrics collection and monitoring capabilities of the Orasi Gateway.

## Overview

The Orasi Gateway includes a comprehensive metrics collection system that provides real-time monitoring and observability for the gateway's performance, health, and operational status. The metrics system is built on top of the Prometheus ecosystem and provides both internal metrics collection and external metrics exposure.

## Features

- **Prometheus Integration**: Native Prometheus metrics format support
- **Real-time Metrics**: Live collection of gateway performance metrics
- **Comprehensive Coverage**: Metrics for requests, responses, errors, and system health
- **Labeled Metrics**: Rich metadata with labels for detailed analysis
- **Automatic Startup/Shutdown**: Integrated with gateway lifecycle
- **Thread-safe**: Concurrent metrics recording support

## Metrics Types

### Counters

Counters are monotonically increasing metrics that track cumulative values:

- `orasi_gateway_requests_total` - Total number of requests processed
- `orasi_gateway_requests_successful_total` - Total number of successful requests
- `orasi_gateway_requests_failed_total` - Total number of failed requests
- `orasi_gateway_rate_limit_violations_total` - Total number of rate limit violations
- `orasi_gateway_circuit_breaker_trips_total` - Total number of circuit breaker trips
- `orasi_gateway_service_discovery_events_total` - Total number of service discovery events
- `orasi_gateway_load_balancer_events_total` - Total number of load balancer events
- `orasi_gateway_health_checks_total` - Total number of health checks performed

### Gauges

Gauges represent current values that can go up or down:

- `orasi_gateway_active_connections` - Current number of active connections
- `orasi_gateway_up` - Gateway health status (1 = up, 0 = down)

### Histograms

Histograms track the distribution of values over time:

- `orasi_gateway_response_time_ms` - Response time distribution in milliseconds

## Usage

### Basic Usage

```rust
use orasi_gateway::{config::GatewayConfig, metrics::MetricsCollector};

// Create configuration
let config = GatewayConfig::default();

// Create metrics collector
let collector = MetricsCollector::new(&config).await?;

// Start metrics collection
collector.start().await?;

// Record metrics
collector.record_request("GET", "/api/v1/health", 200);
collector.record_response_time("GET", "/api/v1/health", 15.5);
collector.record_active_connections(42);

// Stop metrics collection
collector.stop().await?;
```

### Recording Different Types of Metrics

```rust
// Record requests with different status codes
collector.record_request("GET", "/api/v1/users", 200);
collector.record_request("POST", "/api/v1/users", 201);
collector.record_request("DELETE", "/api/v1/users/123", 404);

// Record response times
collector.record_response_time("GET", "/api/v1/health", 15.5);
collector.record_response_time("POST", "/api/v1/users", 45.2);

// Record system events
collector.record_rate_limit_violation("192.168.1.1");
collector.record_circuit_breaker_trip("user-service");
collector.record_service_discovery_event("service_added", "user-service");
collector.record_load_balancer_event("endpoint_selected", "user-service");
collector.record_health_check("user-service", true);
```

### Integration with Gateway

The metrics collector is automatically integrated with the Orasi Gateway:

```rust
use orasi_gateway::{init_gateway, GatewayConfig};

// Initialize gateway (includes metrics collector)
let config = GatewayConfig::default();
let mut gateway = init_gateway(config).await?;

// Start gateway (starts metrics collection)
gateway.start().await?;

// Gateway automatically records metrics during operation
// ...

// Shutdown gateway (stops metrics collection)
gateway.shutdown().await?;
```

## Configuration

Metrics configuration is part of the main gateway configuration:

```rust
let config = GatewayConfig {
    gateway_id: "my-gateway".to_string(),
    metrics_endpoint: "0.0.0.0:9090".to_string(),
    // ... other configuration
};
```

### Configuration Options

- `metrics_endpoint`: The endpoint where Prometheus metrics are exposed (default: "0.0.0.0:9090")
- `gateway_id`: Unique identifier for the gateway instance

## Prometheus Integration

The metrics are automatically exposed in Prometheus format at the configured metrics endpoint. You can scrape these metrics using Prometheus or any other monitoring system that supports Prometheus format.

### Example Prometheus Configuration

```yaml
scrape_configs:
  - job_name: 'orasi-gateway'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

### Available Metrics

When you access the metrics endpoint, you'll see metrics like:

```
# HELP orasi_gateway_requests_total Total number of requests
# TYPE orasi_gateway_requests_total counter
orasi_gateway_requests_total{method="GET",path="/api/v1/health"} 42

# HELP orasi_gateway_response_time_ms Response time in milliseconds
# TYPE orasi_gateway_response_time_ms histogram
orasi_gateway_response_time_ms_bucket{method="GET",path="/api/v1/health",le="10"} 15
orasi_gateway_response_time_ms_bucket{method="GET",path="/api/v1/health",le="50"} 42
orasi_gateway_response_time_ms_bucket{method="GET",path="/api/v1/health",le="+Inf"} 42

# HELP orasi_gateway_active_connections Current number of active connections
# TYPE orasi_gateway_active_connections gauge
orasi_gateway_active_connections 42
```

## Testing

The metrics implementation includes comprehensive tests:

```bash
# Run all metrics tests
cargo test metrics::tests

# Run specific test
cargo test metrics::tests::test_metrics_collector_creation
```

## Example

See `examples/metrics_example.rs` for a complete example demonstrating all metrics functionality.

## Error Handling

The metrics collector includes proper error handling:

- Graceful startup/shutdown
- Error reporting for failed operations
- State validation (e.g., preventing metrics collection when not running)
- Logging of all operations for debugging

## Performance Considerations

- Metrics recording is designed to be lightweight and non-blocking
- All metrics operations are thread-safe
- Prometheus metrics are exposed on a separate endpoint to avoid impacting main gateway performance
- Metrics collection uses efficient data structures and minimal memory overhead

## Troubleshooting

### Common Issues

1. **Metrics endpoint not accessible**: Check that the metrics endpoint is correctly configured and not blocked by firewall
2. **No metrics appearing**: Ensure the metrics collector is started before recording metrics
3. **High memory usage**: Monitor metrics cardinality and consider reducing label values

### Debugging

Enable debug logging to see detailed metrics operations:

```rust
use tracing::Level;

tracing_subscriber::fmt()
    .with_max_level(Level::DEBUG)
    .init();
```

## Future Enhancements

- Custom metrics support
- Metrics aggregation and summarization
- Integration with additional monitoring systems
- Metrics persistence and historical analysis
- Advanced alerting and anomaly detection
