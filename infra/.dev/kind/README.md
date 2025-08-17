# Kind Cluster for Orasi Testing

This directory contains the configuration and scripts for setting up a local Kubernetes cluster using Kind (Kubernetes in Docker) for testing the Orasi observability platform.

## 🚀 Quick Start

### Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [Kind](https://kind.sigs.k8s.io/docs/user/quick-start/#installation)
- [kubectl](https://kubernetes.io/docs/tasks/tools/install-kubectl/)
- [Just](https://github.com/casey/just) (optional, for build automation)

### Setup

#### Option 1: Using Just (Recommended)

```bash
# Quick start with Just
just kind-quickstart

# Or step by step
just kind-setup
just kind-test
just kind-info
```

#### Option 2: Manual Setup

1. **Clone and navigate to the project:**
   ```bash
   cd infra/.dev/kind
   ```

2. **Make scripts executable:**
   ```bash
   chmod +x setup.sh teardown.sh
   ```

3. **Run the setup script:**
   ```bash
   ./setup.sh
   ```

The setup script will:
- Create a Kind cluster with 3 nodes (1 control plane + 2 workers)
- Install Calico CNI for networking
- Install NGINX Ingress Controller
- Install Metrics Server
- Build and load Orasi Docker images
- Deploy Orasi services to the cluster

### Access Services

After setup, you can access the services at:

- **Bridge API**: http://localhost:30080
- **Schema Registry**: http://localhost:30081
- **Web UI**: http://localhost:30082

### Cleanup

#### Option 1: Using Just (Recommended)

```bash
# Teardown cluster
just kind-teardown

# Full cleanup (cluster + Docker + images)
just kind-teardown-full
```

#### Option 2: Manual Cleanup

To clean up the cluster and resources:

```bash
./teardown.sh
```

This will present you with cleanup options:
1. Delete namespace only
2. Delete cluster only
3. Delete cluster and clean up Docker resources
4. Full cleanup (cluster + Docker + images)
5. Cancel

## 📋 Cluster Configuration

### Cluster Specs

- **Name**: `orasi-test`
- **Nodes**: 3 (1 control plane + 2 workers)
- **Kubernetes Version**: v1.28.0
- **CNI**: Calico
- **Ingress**: NGINX Ingress Controller

### Port Mappings

The control plane node exposes the following ports:

| Service | Container Port | Host Port | Protocol |
|---------|---------------|-----------|----------|
| HTTP | 80 | 8080 | TCP |
| HTTPS | 443 | 8443 | TCP |
| Bridge API | 30080 | 30080 | TCP |
| Schema Registry | 30081 | 30081 | TCP |
| Web UI | 30082 | 30082 | TCP |

### Resource Limits

| Service | CPU Request | CPU Limit | Memory Request | Memory Limit |
|---------|-------------|-----------|----------------|--------------|
| Bridge API | 100m | 500m | 128Mi | 512Mi |
| Schema Registry | 100m | 500m | 128Mi | 512Mi |
| Web UI | 50m | 200m | 64Mi | 256Mi |

## 🛠️ Development Workflow

### Just Commands

The project includes comprehensive Just targets for managing the Kind cluster:

```bash
# Setup and teardown
just kind-setup              # Setup cluster and deploy services
just kind-teardown           # Teardown cluster
just kind-teardown-full      # Full cleanup (cluster + Docker + images)
just kind-quickstart         # Quick setup and test

# Testing and monitoring
just kind-test               # Test cluster health
just kind-status             # Show cluster status
just kind-info               # Show cluster information

# Logs and debugging
just kind-logs               # View all service logs
just kind-logs-bridge-api    # View bridge-api logs
just kind-logs-schema-registry # View schema-registry logs
just kind-logs-web-ui        # View web-ui logs

# Service management
just kind-restart            # Restart all services
just kind-restart-service <service> # Restart specific service
just kind-scale <service> <replicas> # Scale service
just kind-scale-all <replicas> # Scale all services

# Port forwarding
just kind-port-forward       # Port forward all services
just kind-port-forward-bridge-api # Port forward bridge-api
just kind-port-forward-schema-registry # Port forward schema-registry
just kind-port-forward-web-ui # Port forward web-ui

# Health checks
just kind-health             # Test all health endpoints
just kind-health-bridge-api  # Test bridge-api health
just kind-health-schema-registry # Test schema-registry health
just kind-health-web-ui      # Test web-ui health

# Browser access
just kind-open               # Open all services in browser
just kind-open-bridge-api    # Open bridge-api in browser
just kind-open-schema-registry # Open schema-registry in browser
just kind-open-web-ui        # Open web-ui in browser

# Image management
just kind-build-images       # Build and load all images
just kind-build-image <service> # Build and load specific image
just kind-deploy             # Deploy services
just kind-delete             # Delete services
```

### Building Images

The setup script automatically builds and loads Docker images. To rebuild images manually:

```bash
# Using Just (recommended)
just docker-bake-all

# Or manually
docker build -f infra/docker/Dockerfile -t orasi/bridge-api:latest .
docker build -f infra/docker/Dockerfile -t orasi/schema-registry:latest .
docker build -f infra/docker/Dockerfile -t orasi/web:latest .
```

### Loading Images

```bash
kind load docker-image orasi/bridge-api:latest --name orasi-test
kind load docker-image orasi/schema-registry:latest --name orasi-test
kind load docker-image orasi/web:latest --name orasi-test
```

### Deploying Services

```bash
kubectl apply -f k8s/ -n orasi
```

### Monitoring Services

```bash
# Check pod status
kubectl get pods -n orasi

# View logs
kubectl logs -f deployment/bridge-api -n orasi
kubectl logs -f deployment/schema-registry -n orasi
kubectl logs -f deployment/web-ui -n orasi

# Check services
kubectl get services -n orasi

# Check ingress
kubectl get ingress -n orasi
```

## 🔧 Configuration

### Environment Variables

Key environment variables for the services:

```bash
# Logging
RUST_LOG=info

# Configuration paths
CONFIG_PATH=/config/bridge-api-config.yaml
CONFIG_PATH=/config/schema-registry-config.yaml
CONFIG_PATH=/config/web-config.yaml
```

### Configuration Files

Configuration is managed through Kubernetes ConfigMaps:

- **Bridge API**: `/config/bridge-api-config.yaml`
- **Schema Registry**: `/config/schema-registry-config.yaml`
- **Web UI**: `/config/web-config.yaml`

To update configuration:

1. Edit the ConfigMap in `k8s/configmap.yaml`
2. Apply the changes: `kubectl apply -f k8s/configmap.yaml`
3. Restart the affected pods: `kubectl rollout restart deployment/<service-name> -n orasi`

## 🧪 Testing

### Health Checks

All services include health check endpoints:

- **Bridge API**: `/health/live`, `/health/ready`
- **Schema Registry**: `/health/live`, `/health/ready`
- **Web UI**: `/health`

### Load Testing

```bash
# Generate test data
cargo run -p data-generator

# Run load tests
cargo run -p test-runner
```

### Integration Testing

```bash
# Run integration tests
cargo test --package bridge-api --test integration
cargo test --package schema-registry --test integration
```

## 🔍 Troubleshooting

### Common Issues

#### Cluster Creation Fails

```bash
# Check Docker is running
docker info

# Check available resources
docker system df

# Clean up Docker resources
docker system prune -a
```

#### Services Not Starting

```bash
# Check pod status
kubectl get pods -n orasi

# Check pod events
kubectl describe pod <pod-name> -n orasi

# Check logs
kubectl logs <pod-name> -n orasi
```

#### Images Not Found

```bash
# Check if images exist
docker images | grep orasi

# Rebuild and load images
./setup.sh
```

#### Port Already in Use

```bash
# Check what's using the port
lsof -i :30080
lsof -i :30081
lsof -i :30082

# Kill the process or change ports in kind-config.yaml
```

### Debugging Commands

```bash
# Get cluster info
kind get clusters
kubectl cluster-info

# Check nodes
kubectl get nodes

# Check all resources in namespace
kubectl get all -n orasi

# Check ingress controller
kubectl get pods -n ingress-nginx

# Check Calico
kubectl get pods -n kube-system | grep calico
```

### Logs and Monitoring

```bash
# Follow logs for all services
kubectl logs -f -l app=bridge-api -n orasi
kubectl logs -f -l app=schema-registry -n orasi
kubectl logs -f -l app=web-ui -n orasi

# Check metrics
kubectl top pods -n orasi
kubectl top nodes
```

## 📚 Additional Resources

- [Kind Documentation](https://kind.sigs.k8s.io/)
- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Calico Documentation](https://docs.tigera.io/calico/)
- [NGINX Ingress Documentation](https://kubernetes.github.io/ingress-nginx/)

## 🤝 Contributing

When contributing to the Kind setup:

1. **Test locally** before submitting changes
2. **Update documentation** for any configuration changes
3. **Consider resource requirements** for the cluster
4. **Follow Kubernetes best practices** for manifests

## 📄 License

This project is licensed under the same terms as the main Orasi project.
