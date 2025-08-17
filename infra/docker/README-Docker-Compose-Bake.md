# Docker Compose with Docker Bake Integration

This document explains how to use Docker Compose with the Docker Bake configuration to run the complete Orasi stack.

## Overview

The integration provides two main compose files:

- `docker-compose.bake.yml` - Production setup with release images
- `docker-compose.bake.dev.yml` - Development setup with debug images

## Services Included

### Orasi Services (Built with Docker Bake)
- **bridge-api** - REST/gRPC API server
- **schema-registry** - Schema registry service  
- **controller** - Kubernetes controller

### Infrastructure Services
- **minio** - S3-compatible object storage
- **postgres** - PostgreSQL database
- **redis** - Redis cache
- **spark-master** - Apache Spark master
- **spark-worker** - Apache Spark worker
- **spark-history** - Spark history server

## Quick Start

### Development Environment
```bash
# Build debug images and start all services
just compose-bake-build-up-dev

# Or step by step:
just docker-bake-dev
just compose-bake-up-dev
```

### Production Environment
```bash
# Build release images and start all services
just compose-bake-build-up

# Or step by step:
just docker-bake-all
just compose-bake-up
```

## Service Ports

| Service | Port | Description |
|---------|------|-------------|
| bridge-api | 8080 | REST API |
| bridge-api | 4317 | gRPC |
| bridge-api | 4318 | gRPC (HTTP) |
| bridge-api | 9090 | Metrics |
| schema-registry | 8081 | REST API |
| schema-registry | 9091 | Metrics |
| controller | 8082 | REST API |
| controller | 9092 | Metrics |
| minio | 9000 | API |
| minio | 9001 | Console |
| postgres | 5432 | Database |
| redis | 6379 | Cache |
| spark-master | 8083 | Web UI |
| spark-worker | 8084 | Web UI |
| spark-history | 18080 | History Server |

## Environment Variables

### Build Configuration
- `BUILD_TYPE` - Build type (debug/release, default: release)
- `RUST_LOG` - Log level (default: info for release, debug for debug builds)
- `RUST_BACKTRACE` - Enable backtraces (default: 1)

### Service Configuration
- `DATABASE_URL` - PostgreSQL connection string
- `REDIS_URL` - Redis connection string
- `MINIO_ENDPOINT` - MinIO endpoint
- `MINIO_ACCESS_KEY` - MinIO access key
- `MINIO_SECRET_KEY` - MinIO secret key
- `PORT` - Service port (default: 8080)

## Justfile Commands

### Basic Operations
```bash
# Start services
just compose-bake-up              # Release mode
just compose-bake-up-dev          # Debug mode

# Stop services
just compose-bake-down            # Release mode
just compose-bake-down-dev        # Debug mode

# Restart services
just compose-bake-restart         # Release mode
just compose-bake-restart-dev     # Debug mode
```

### Build and Deploy
```bash
# Build and start
just compose-bake-build-up        # Release mode
just compose-bake-build-up-dev    # Debug mode

# Build, push, and start
just compose-bake-build-push-up myregistry v1.0.0
just compose-bake-build-push-up-dev myregistry debug
```

### Logging and Monitoring
```bash
# View logs
just compose-bake-logs            # All services
just compose-bake-logs-service bridge-api  # Specific service

# Follow logs
just compose-bake-logs-follow     # All services
just compose-bake-logs-follow-service bridge-api  # Specific service

# Service status
just compose-bake-status          # Release mode
just compose-bake-status-dev      # Debug mode
```

### Cleanup
```bash
# Stop and remove volumes
just compose-bake-down-clean      # Release mode
just compose-bake-down-clean-dev  # Debug mode
```

## Development Workflow

### 1. Initial Setup
```bash
# Build debug images
just docker-bake-dev

# Start development environment
just compose-bake-up-dev

# Check status
just compose-bake-status-dev
```

### 2. Development Iteration
```bash
# Make code changes...

# Rebuild specific service
just docker-bake-bridge-api-debug

# Restart service
just compose-bake-restart-dev

# Follow logs
just compose-bake-logs-follow-service-dev bridge-api
```

### 3. Testing
```bash
# Test bridge-api
curl http://localhost:8080/health/live

# Test schema-registry
curl http://localhost:8081/health/live

# Test controller
curl http://localhost:8082/health/live
```

## Production Workflow

### 1. Build and Deploy
```bash
# Build release images
just docker-bake-all

# Start production environment
just compose-bake-up

# Verify deployment
just compose-bake-status
```

### 2. Monitoring
```bash
# Check service health
just health-all

# View metrics
just metrics

# Monitor logs
just compose-bake-logs-follow
```

### 3. Updates
```bash
# Build new images
just docker-bake-all

# Restart services
just compose-bake-restart

# Verify update
just compose-bake-status
```

## Configuration Files

### Volume Mounts
- `./config:/app/config:ro` - Configuration files
- `./target:/app/target:ro` - Debug builds (development only)
- `~/.kube:/app/.kube:ro` - Kubernetes config (controller)

### Data Volumes
- `minio_data` - MinIO object storage
- `postgres_data` - PostgreSQL database
- `spark_data` - Spark work directory
- `*_logs` - Service logs
- `*_data` - Service data

## Health Checks

All services include health checks:

```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8080/health/live"]
  interval: 30s
  timeout: 10s
  retries: 3
```

## Troubleshooting

### Common Issues

1. **Port Conflicts**
   ```bash
   # Check what's using a port
   lsof -i :8080
   
   # Use different ports in compose file
   ports:
     - "8085:8080"  # Map to different host port
   ```

2. **Image Not Found**
   ```bash
   # Rebuild images
   just docker-bake-all
   
   # Check available images
   just docker-bake-images
   ```

3. **Service Won't Start**
   ```bash
   # Check logs
   just compose-bake-logs-service bridge-api
   
   # Check dependencies
   just compose-bake-status
   ```

4. **Database Connection Issues**
   ```bash
   # Check postgres health
   docker exec orasi-postgres pg_isready -U postgres
   
   # Check database URL
   echo $DATABASE_URL
   ```

### Debug Mode

For debugging issues:

```bash
# Use debug images
just compose-bake-up-dev

# Enable verbose logging
RUST_LOG=debug just compose-bake-up-dev

# Follow logs
just compose-bake-logs-follow-dev
```

### Clean Slate

To start fresh:

```bash
# Stop everything
just compose-bake-down-clean-dev

# Remove images
just docker-bake-clean

# Rebuild and start
just compose-bake-build-up-dev
```

## Integration with CI/CD

### GitHub Actions Example

```yaml
- name: Build and Deploy
  run: |
    # Build images
    just docker-bake-all
    
    # Push to registry
    just docker-bake-push ghcr.io/${{ github.repository }} ${{ github.sha }}
    
    # Deploy (if using remote compose)
    docker-compose -f infra/docker/docker-compose.bake.yml up -d
```

### Local Development with Remote Images

```bash
# Use remote registry
BUILD_TYPE=release docker-compose -f infra/docker/docker-compose.bake.yml up -d

# Or with custom registry
REGISTRY=myregistry TAG=v1.0.0 docker-compose -f infra/docker/docker-compose.bake.yml up -d
```

## Next Steps

1. **Customize Configuration** - Modify environment variables and volumes
2. **Add Monitoring** - Integrate with Prometheus/Grafana
3. **Set Up Logging** - Configure centralized logging
4. **Security Hardening** - Add secrets management
5. **Scaling** - Configure service replication
