# Bridge Core

The foundational core crate for the Orasi OpenTelemetry Data Lake Bridge, providing essential abstractions, types, and orchestration capabilities.

## Overview

Bridge Core serves as the central nervous system of the Orasi bridge, providing:

- **Core Types and Abstractions**: Common data structures and traits used across the bridge
- **Configuration Management**: Centralized configuration handling and validation
- **Error Handling**: Comprehensive error types and error propagation
- **Health Monitoring**: System health checks and monitoring capabilities
- **Pipeline Orchestration**: Data pipeline management and coordination
- **Metrics and Observability**: Built-in metrics collection and monitoring
- **Utility Functions**: Common utilities for data processing and system operations

## Key Features

### Core Abstractions
- **BridgeResult**: Standardized result type for bridge operations
- **BridgeError**: Comprehensive error types with context
- **Config Traits**: Configuration management interfaces
- **Pipeline Traits**: Data pipeline orchestration interfaces

### Configuration Management
- **Multi-format Support**: TOML, JSON, YAML, and environment variables
- **Validation**: Schema-based configuration validation
- **Hot Reloading**: Runtime configuration updates
- **Environment-specific**: Development, staging, and production configs

### Health Monitoring
- **Health Checks**: System component health monitoring
- **Metrics Collection**: Prometheus metrics integration
- **Status Reporting**: Real-time system status
- **Dependency Monitoring**: External service health tracking

### Pipeline Orchestration
- **Pipeline Management**: Data flow coordination
- **Component Lifecycle**: Start, stop, and restart capabilities
- **Resource Management**: Memory and connection pooling
- **Error Recovery**: Graceful error handling and recovery

## Quick Start

### Basic Usage

```rust
use bridge_core::{
    config::BridgeConfig,
    error::BridgeResult,
    health::HealthMonitor,
    metrics::MetricsRegistry,
    pipeline::PipelineManager,
};

#[tokio::main]
async fn main() -> BridgeResult<()> {
    // Load configuration
    let config = BridgeConfig::load()?;
    
    // Initialize metrics
    let metrics = MetricsRegistry::new();
    
    // Create health monitor
    let health_monitor = HealthMonitor::new();
    
    // Create pipeline manager
    let pipeline_manager = PipelineManager::new(config.pipeline.clone());
    
    // Start health monitoring
    health_monitor.start().await?;
    
    // Start pipeline
    pipeline_manager.start().await?;
    
    // Main event loop
    loop {
        // Check health status
        if let Some(health_status) = health_monitor.get_status() {
            if !health_status.is_healthy() {
                tracing::warn!("System health degraded: {:?}", health_status);
            }
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }
}
```

### Configuration Example

```toml
# bridge.toml
[bridge]
name = "orasi-bridge"
version = "0.1.0"
environment = "development"

[bridge.health]
enabled = true
port = 8081
interval_seconds = 30

[bridge.metrics]
enabled = true
port = 9090
path = "/metrics"

[bridge.pipeline]
max_concurrent_tasks = 100
buffer_size = 10000
timeout_seconds = 300

[bridge.storage]
type = "postgres"
url = "postgres://localhost/orasi_bridge"
pool_size = 10
```

## Architecture

Bridge Core follows a modular architecture with clear separation of concerns:

```
┌─────────────────┐
│   Bridge Core   │
├─────────────────┤
│  Configuration  │
│  Error Handling │
│  Health Monitor │
│  Metrics        │
│  Pipeline Mgmt  │
│  Utilities      │
└─────────────────┘
```

### Core Components

1. **Configuration**: Centralized config management with validation
2. **Error Handling**: Comprehensive error types and propagation
3. **Health Monitor**: System health tracking and reporting
4. **Metrics**: Prometheus metrics collection and export
5. **Pipeline Manager**: Data pipeline orchestration
6. **Utilities**: Common helper functions and types

## Dependencies

### Core Dependencies
- **tokio**: Async runtime
- **serde**: Serialization/deserialization
- **tracing**: Structured logging
- **anyhow**: Error handling
- **thiserror**: Error type generation

### OpenTelemetry
- **opentelemetry**: Core OpenTelemetry functionality
- **opentelemetry-otlp**: OTLP protocol support
- **opentelemetry-semantic-conventions**: Standard semantic conventions

### Networking
- **tonic**: gRPC framework
- **axum**: HTTP web framework
- **hyper**: HTTP client/server
- **tower**: Middleware framework

### Data Processing
- **sqlx**: Database access
- **redis**: Caching and session storage
- **prometheus**: Metrics collection

## Development

### Building

```bash
# Build the crate
cargo build

# Build with optimizations
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_health_monitor

# Run with logging
RUST_LOG=debug cargo test
```

### Documentation

```bash
# Generate documentation
cargo doc

# Open documentation in browser
cargo doc --open
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
