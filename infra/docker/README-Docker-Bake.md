# Docker Bake Configuration for Orasi

This directory contains Docker Bake configuration for building individual service images for the Orasi OpenTelemetry Data Lake Bridge project.

## Overview

The Docker Bake setup allows you to build individual Docker images for each service component:

- **bridge-api**: REST/gRPC API server
- **schema-registry**: Schema registry service
- **controller**: Kubernetes controller

Each service can be built in both debug and release modes.

## Files

- `docker-bake.hcl`: Main Docker Bake configuration file
- `Dockerfile.builder`: Multi-stage builder that compiles the entire workspace
- `Dockerfile.service`: Template for individual service images
- `Dockerfile.runtime`: Base runtime environment
- `build.sh`: Convenience script for building images

## Prerequisites

- Docker with Buildx support
- Docker Buildx driver configured (preferably `docker-container`)

```bash
# Check if buildx is available
docker buildx version

# Create a new builder instance if needed
docker buildx create --name mybuilder --use
docker buildx inspect --bootstrap
```

## Quick Start

### Build All Services (Release Mode)

```bash
# Using the build script
./infra/docker/build.sh

# Or directly with docker buildx bake
docker buildx bake all
```

### Build Individual Services

```bash
# Build only bridge-api
./infra/docker/build.sh bridge-api

# Build only schema-registry in debug mode
./infra/docker/build.sh -t debug schema-registry

# Build only controller
./infra/docker/build.sh controller
```

### Build Development Images (Debug Mode)

```bash
# Build all services in debug mode
./infra/docker/build.sh dev

# Or build specific service in debug mode
./infra/docker/build.sh -t debug bridge-api
```

## Configuration Options

### Environment Variables

You can customize the build by setting environment variables:

```bash
export BUILD_TYPE=debug          # debug or release
export REGISTRY=myregistry       # Docker registry name
export TAG=v1.0.0               # Image tag
export PLATFORMS=linux/amd64    # Target platforms
```

### Command Line Options

The build script supports various options:

```bash
./infra/docker/build.sh -h              # Show help
./infra/docker/build.sh -t debug        # Set build type
./infra/docker/build.sh -r myregistry   # Set registry
./infra/docker/build.sh --tag v1.0.0   # Set tag
./infra/docker/build.sh -p linux/arm64 # Set platform
```

## Available Targets

### Groups

- `all`: Build all services (debug and release)
- `dev`: Build debug versions of all services
- `prod`: Build release versions of all services
- `bridge-api`: Build bridge-api service
- `schema-registry`: Build schema-registry service
- `controller`: Build controller service

### Individual Targets

- `bridge-api-debug`: Debug build of bridge-api
- `bridge-api-release`: Release build of bridge-api
- `schema-registry-debug`: Debug build of schema-registry
- `schema-registry-release`: Release build of schema-registry
- `controller-debug`: Debug build of controller
- `controller-release`: Release build of controller

## Examples

### Basic Usage

```bash
# Build all release images
docker buildx bake all

# Build all debug images
docker buildx bake dev

# Build specific service
docker buildx bake bridge-api-release
```

### Custom Registry and Tags

```bash
# Build with custom registry
docker buildx bake --set "*.tags=myregistry/*:latest" all

# Build with specific tag
docker buildx bake --set "*.tags=orasi/*:v1.0.0" prod

# Build for multiple platforms
docker buildx bake --set "*.platforms=linux/amd64,linux/arm64" all
```

### Using the Build Script

```bash
# Build all services with custom settings
./infra/docker/build.sh -r myregistry --tag v1.0.0 -p "linux/amd64,linux/arm64" all

# Build debug versions
./infra/docker/build.sh -t debug dev

# Build specific service
./infra/docker/build.sh -t release bridge-api
```

## Image Structure

Each service image follows this structure:

```
/app/
├── {service-name}          # The compiled binary
├── logs/                   # Log directory
└── data/                   # Data directory
```

### Runtime Environment

- **Base Image**: `debian:bookworm-slim`
- **User**: `orasi` (non-root)
- **Working Directory**: `/app`
- **Default Port**: `8080` (configurable via `PORT` environment variable)

### Health Checks

Each service includes a health check that attempts to connect to `/health/live` endpoint:

```dockerfile
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:${PORT}/health/live || exit 1
```

## Caching

The build configuration includes multiple caching strategies:

- **GitHub Actions Cache**: For CI/CD environments
- **Local Cache**: For development builds
- **Layer Caching**: Optimized dependency builds

### Cache Management

```bash
# Clear local cache
rm -rf /tmp/.buildx-cache*

# Use specific cache
docker buildx bake --set "*.cache-from=type=local,src=/path/to/cache" all
```

## Troubleshooting

### Common Issues

1. **Buildx not available**: Install Docker Buildx
2. **Out of memory**: Increase Docker memory limits
3. **Cache issues**: Clear cache and rebuild
4. **Platform issues**: Ensure target platform is supported

### Debug Builds

```bash
# Enable verbose output
docker buildx bake --progress=plain all

# Build with debug information
./infra/docker/build.sh -t debug all
```

### Performance Optimization

```bash
# Use parallel builds
docker buildx bake --parallel=4 all

# Use specific builder
docker buildx bake --builder=mybuilder all
```

## Integration with CI/CD

### GitHub Actions Example

```yaml
- name: Build Docker Images
  run: |
    docker buildx bake \
      --set "*.platforms=linux/amd64,linux/arm64" \
      --set "*.tags=ghcr.io/${{ github.repository }}/*:${{ github.sha }}" \
      --push \
      all
```

### Local Development

```bash
# Build and run locally
./infra/docker/build.sh -t debug bridge-api
docker run -p 8080:8080 orasi/bridge-api:debug

# Build and push to registry
./infra/docker/build.sh -r myregistry --tag v1.0.0 all
docker push myregistry/bridge-api:v1.0.0
```

## Security Considerations

- All images run as non-root user (`orasi`)
- Minimal base image (`debian:bookworm-slim`)
- No unnecessary packages installed
- Health checks for monitoring
- Configurable ports and environment variables

## Next Steps

1. Customize the Dockerfiles for your specific needs
2. Add service-specific configuration files
3. Set up monitoring and logging
4. Configure deployment manifests
5. Set up CI/CD pipelines
