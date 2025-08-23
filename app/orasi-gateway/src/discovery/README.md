# Service Discovery Implementation

This module provides a comprehensive service discovery system for the Orasi Gateway, supporting multiple backends and health checking capabilities.

## Overview

The service discovery system consists of several components:

- **Core Service Discovery**: Main orchestrator that manages service discovery lifecycle
- **Backends**: Pluggable service discovery backends (etcd, Consul, static)
- **Registry**: Local service registry with health checking
- **Health Checker**: Endpoint health monitoring with retry logic

## Architecture

```
ServiceDiscovery (Core)
├── ServiceRegistry (Local cache)
├── ServiceDiscoveryBackend (etcd/Consul/Static)
└── HealthChecker (Endpoint monitoring)
```

## Components

### Core Service Discovery

The `ServiceDiscovery` struct is the main entry point for service discovery functionality:

```rust
use orasi_gateway::discovery::ServiceDiscovery;
use orasi_gateway::config::GatewayConfig;

let config = GatewayConfig::default();
let mut service_discovery = ServiceDiscovery::new(&config).await?;
service_discovery.start().await?;
```

### Backends

#### Etcd Backend

Supports service discovery using etcd as the backend:

```rust
use orasi_gateway::discovery::backends::EtcdBackend;

let backend = EtcdBackend::new(
    vec!["http://localhost:2379".to_string()],
    "/orasi".to_string()
);
```

#### Consul Backend

Supports service discovery using Consul:

```rust
use orasi_gateway::discovery::backends::ConsulBackend;

let backend = ConsulBackend::new(
    "dc1".to_string(),
    None // Optional token
);
```

#### Static Backend

For static service configuration:

```rust
use orasi_gateway::discovery::backends::StaticBackend;

let services = HashMap::new();
let backend = StaticBackend::new(services);
```

### Health Checking

The system includes two health checker implementations:

#### Basic Health Checker

Simple endpoint health checking:

```rust
use orasi_gateway::discovery::health::HealthChecker;

let health_checker = HealthChecker::new(
    Duration::from_secs(5),
    "/health".to_string()
);
```

#### Advanced Health Checker

Advanced health checking with retry logic and thresholds:

```rust
use orasi_gateway::discovery::health::{AdvancedHealthChecker, HealthCheckConfig};

let config = HealthCheckConfig {
    timeout: Duration::from_secs(5),
    health_check_path: "/health".to_string(),
    interval: Duration::from_secs(30),
    retries: 3,
    success_threshold: 2,
    failure_threshold: 3,
};

let health_checker = AdvancedHealthChecker::new(config);
```

## Usage

### Basic Usage

```rust
use orasi_gateway::{
    config::GatewayConfig,
    discovery::ServiceDiscovery,
    types::{ServiceInfo, ServiceEndpoint, ServiceHealthStatus, EndpointHealthStatus},
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration
    let mut config = GatewayConfig::default();
    config.service_discovery.backend = ServiceDiscoveryBackend::Static;
    
    // Initialize service discovery
    let mut service_discovery = ServiceDiscovery::new(&config).await?;
    service_discovery.start().await?;
    
    // Register a service
    let service = ServiceInfo {
        name: "my-service".to_string(),
        endpoints: vec![ServiceEndpoint {
            url: "http://localhost:8080".to_string(),
            weight: 1,
            health_status: EndpointHealthStatus::Healthy,
            metadata: HashMap::new(),
        }],
        health_status: ServiceHealthStatus::Healthy,
        metadata: HashMap::new(),
    };
    
    service_discovery.register_service("my-service-1", &service).await?;
    
    // Get services
    let services = service_discovery.get_services().await;
    let healthy_services = service_discovery.get_healthy_services().await;
    
    // Stop service discovery
    service_discovery.stop().await?;
    
    Ok(())
}
```

### Configuration

Service discovery can be configured through the `GatewayConfig`:

```rust
let mut config = GatewayConfig::default();
config.service_discovery.backend = ServiceDiscoveryBackend::Etcd;
config.service_discovery.endpoints = vec!["http://localhost:2379".to_string()];
config.service_discovery.refresh_interval = Duration::from_secs(30);
config.service_discovery.health_check_interval = Duration::from_secs(10);
```

## Features

- **Multiple Backends**: Support for etcd, Consul, and static configuration
- **Health Checking**: Automatic endpoint health monitoring
- **Retry Logic**: Configurable retry mechanisms for health checks
- **Background Processing**: Automatic service refresh and health checking
- **Thread Safety**: Full async/await support with proper synchronization
- **Extensible**: Easy to add new backends or health check strategies

## Health Check Configuration

The health checker supports various configuration options:

- **Timeout**: Maximum time to wait for health check response
- **Retries**: Number of retry attempts for failed health checks
- **Success Threshold**: Number of consecutive successful checks required
- **Failure Threshold**: Number of consecutive failures before marking unhealthy
- **Health Check Path**: HTTP endpoint to check (default: `/health`)

## Service States

Services can be in the following health states:

- **Healthy**: Service is fully operational
- **Degraded**: Service is partially operational
- **Unhealthy**: Service is not operational
- **Unknown**: Service health status is unknown

## Error Handling

The service discovery system provides comprehensive error handling:

- Backend connection failures
- Health check timeouts
- Service registration/deregistration errors
- Configuration validation errors

All errors are wrapped in `GatewayError` with specific error types for different failure scenarios.

## Examples

See the `examples/service_discovery_example.rs` file for a complete working example of the service discovery system.
