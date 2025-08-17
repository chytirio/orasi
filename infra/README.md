# Infrastructure

This directory contains all infrastructure-related components for the Orasi observability platform, including Docker configurations, Kubernetes controllers, and development tools.

## 🐳 Docker Infrastructure

The `docker/` directory contains all Docker-related configurations for building, running, and deploying Orasi services.

### Key Components

- **Multi-stage Dockerfiles**: Optimized builds for different environments
- **Docker Compose**: Development and production orchestration
- **Docker Bake**: Advanced build automation with caching
- **Build Scripts**: Automated build and deployment tools

### Quick Start

```bash
# Build all services
just docker-bake-all

# Start development environment
just compose-bake-up-dev

# Start production environment
just compose-bake-up
```

For detailed Docker documentation, see [docker/README-Docker-Bake.md](docker/README-Docker-Bake.md) and [docker/README-Docker-Compose-Bake.md](docker/README-Docker-Compose-Bake.md).

## 🎛️ Kubernetes Controller

The `controller/` directory contains a Kubernetes operator for managing Orasi instances in Kubernetes clusters.

### Features

- **Custom Resource Definitions**: Define Orasi instances as Kubernetes resources
- **Reconciliation Logic**: Automatically manage service deployment and configuration
- **Health Monitoring**: Monitor and report on service health
- **Configuration Management**: Handle configuration updates and rollouts

### Usage

```bash
# Build the controller
cargo build -p controller

# Deploy to Kubernetes
kubectl apply -f controller/yaml/
```

For detailed controller documentation, see [controller/README.md](controller/README.md).

## 🐳 Kind Cluster for Local Testing

The `.dev/kind/` directory contains a complete setup for local Kubernetes testing using Kind (Kubernetes in Docker).

### Features

- **Multi-node cluster**: 1 control plane + 2 worker nodes
- **Production-like environment**: Includes Calico CNI, NGINX Ingress, and Metrics Server
- **Automated deployment**: Complete setup and teardown scripts
- **Service isolation**: Dedicated namespace for Orasi services
- **Port forwarding**: Direct access to services via NodePort

### Quick Start

```bash
# Navigate to kind directory
cd infra/.dev/kind

# Setup cluster and deploy services
./setup.sh

# Access services
# Bridge API: http://localhost:30080
# Schema Registry: http://localhost:30081
# Web UI: http://localhost:30082

# Cleanup
./teardown.sh
```

For detailed Kind cluster documentation, see [.dev/kind/README.md](.dev/kind/README.md).

## 🛠️ Development Tools

### Build Automation

The infrastructure includes comprehensive build automation:

- **Docker Bake**: Multi-platform image building with caching
- **Just Commands**: Simplified development workflows
- **CI/CD Integration**: GitHub Actions and other CI system support

### Environment Management

- **Development Environment**: Local development with hot reloading
- **Testing Environment**: Isolated testing with Delta Lake
- **Production Environment**: Optimized for production workloads

## 🚀 Quick Start

### Prerequisites

- Docker and Docker Compose
- Kubernetes cluster (for controller)
- Rust toolchain (for development)

### Development Setup

```bash
# Start development environment
just compose-bake-up-dev

# Build and deploy controller
cargo build -p controller
kubectl apply -f controller/yaml/

# Monitor services
just compose-bake-logs-follow-dev
```

### Kind Cluster Testing

```bash
# Setup Kind cluster for local testing
cd infra/.dev/kind
./setup.sh

# Access services
# Bridge API: http://localhost:30080
# Schema Registry: http://localhost:30081
# Web UI: http://localhost:30082

# Cleanup when done
./teardown.sh
```

### Production Deployment

```bash
# Build production images
just docker-bake-prod

# Deploy to production
just compose-bake-up

# Monitor production services
just compose-bake-logs-follow
```

## 📊 Monitoring and Observability

### Health Checks

All services include health check endpoints:

- **Bridge API**: `http://localhost:8080/health/live`
- **Schema Registry**: `http://localhost:8080/health/live`
- **Controller**: Kubernetes health probes

### Metrics

Services expose Prometheus metrics on `/metrics` endpoints for monitoring and alerting.

### Logging

Structured logging is configured for all services with configurable log levels.

## 🔧 Configuration

### Environment Variables

Key environment variables for infrastructure:

```bash
# Build configuration
BUILD_TYPE=release          # debug or release
REGISTRY=orasi             # Docker registry
TAG=latest                 # Image tag
PLATFORMS=linux/amd64      # Target platforms

# Runtime configuration
RUST_LOG=info              # Log level
PORT=8080                  # Service port
DATABASE_URL=postgres://... # Database connection
```

### Configuration Files

- **Docker Compose**: Service orchestration and networking
- **Docker Bake**: Build configuration and caching
- **Kubernetes Manifests**: Controller deployment and CRDs

## 🧪 Testing

### Integration Testing

```bash
# Start testing environment
just delta-up

# Run integration tests
just delta-test

# Clean up
just delta-down-clean
```

### Load Testing

```bash
# Generate test data
cargo run -p data-generator

# Run load tests
cargo run -p test-runner
```

## 🔒 Security

### Best Practices

- **Non-root containers**: All services run as non-root users
- **Minimal base images**: Using Debian slim images
- **Secrets management**: Kubernetes secrets for sensitive data
- **Network policies**: Isolated service communication

### Security Scanning

```bash
# Scan Docker images
docker scan orasi/bridge-api:latest

# Check for vulnerabilities
cargo audit
```

## 📚 Documentation

- [Docker Documentation](docker/README-Docker-Bake.md)
- [Controller Documentation](controller/README.md)
- [Development Guide](../docs/development.md)
- [Deployment Guide](../docs/deployment.md)

## 🤝 Contributing

When contributing to infrastructure:

1. **Follow Docker best practices** for image optimization
2. **Test locally** before submitting changes
3. **Update documentation** for any configuration changes
4. **Consider security implications** of infrastructure changes

## 📄 License

This project is licensed under the same terms as the main Orasi project.
