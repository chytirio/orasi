# Orasi Controller

A Kubernetes controller for the Orasi OpenTelemetry Data Lakehouse, built with Rust and the kube-rs framework.

## Overview

The Orasi Controller manages custom Document resources in Kubernetes, providing:
- Resource reconciliation and status management
- Prometheus metrics collection
- HTTP API for monitoring and debugging
- Graceful shutdown handling

## Features

- **Document Resource Management**: Manages custom Document CRDs with content, metadata, and visibility controls
- **Observability**: Built-in Prometheus metrics
- **HTTP API**: REST endpoints for health checks, metrics, and debug information
- **Graceful Shutdown**: Proper signal handling and resource cleanup
- **Error Handling**: Comprehensive error handling with retry logic

## Installation

### Prerequisites

- Rust toolchain
- Kubernetes cluster (for full deployment)

### CRD Installation

Apply the Custom Resource Definition:

```bash
# Generate and apply CRD
cargo run --bin crdgen | kubectl apply -f -
```

### Controller Deployment

Build and deploy the controller:

```bash
# Build the controller
cargo build --release

# Run locally (for development)
cargo run --bin controller

# Or deploy to cluster (see deployment examples below)
```

## Usage

### Creating Document Resources

Create a Document resource:

```yaml
apiVersion: orasi.io/v1alpha1
kind: Document
metadata:
  name: my-document
  namespace: default
spec:
  content: "This is my document content"
  hidden: false
  metadata:
    title: "My Document"
    author: "John Doe"
    tags:
      - "important"
      - "documentation"
```

### Monitoring

The controller exposes several HTTP endpoints:

- `GET /` - Debug information and service status
- `GET /health` - Health check endpoint
- `GET /metrics` - Prometheus metrics

Example usage:

```bash
# Get service status
curl http://localhost:8080/

# Get metrics
curl http://localhost:8080/metrics

# Health check
curl http://localhost:8080/health
```

### Metrics

The controller exposes the following Prometheus metrics:

- `doc_controller_reconcile_duration_seconds` - Histogram of reconciliation durations
- `doc_controller_reconciliations_total` - Counter of total reconciliations
- `doc_controller_reconciliation_errors_total` - Counter of reconciliation errors

## Development

### Building

```bash
# Build all binaries
cargo build

# Build specific binary
cargo build --bin controller
cargo build --bin crdgen
```

### Testing

```bash
# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo test
```

### Local Development

```bash
# Run controller locally
RUST_LOG=info cargo run --bin controller

# Generate CRD
cargo run --bin crdgen

# Apply example resources
kubectl apply -f yaml/instance-lorem.yaml
kubectl apply -f yaml/instance-hidden.yaml
```

## Configuration

### Environment Variables

- `RUST_LOG` - Logging level (default: info)

### Kubernetes Configuration

The controller uses the default Kubernetes client configuration:
- In-cluster: Uses service account token
- Local development: Uses kubeconfig file

## Architecture

### Components

1. **Controller**: Main reconciliation logic for Document resources
2. **Reconciler**: Handles individual resource reconciliation
3. **Metrics**: Prometheus metrics collection
4. **HTTP Server**: REST API for monitoring

### Resource Flow

```
Document CR → Controller → Reconciler → Status Update → Event Creation
     ↓              ↓           ↓            ↓              ↓
   Watcher    Error Policy   Metrics    Kubernetes    Logging
```

## Troubleshooting

### Common Issues

1. **CRD not found**: Ensure CRD is applied before running controller
2. **Permission denied**: Check RBAC permissions for the service account
3. **Metrics not available**: Verify Prometheus metrics endpoint is accessible

### Debugging

Enable debug logging:

```bash
RUST_LOG=debug cargo run --bin controller
```

Check controller logs:

```bash
kubectl logs -l app=orasi-controller
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

Apache-2.0 License - see LICENSE file for details.
