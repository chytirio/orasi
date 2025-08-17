# Docker Bake Justfile Targets

Quick reference for Docker Bake commands using justfile.

## 🚀 Quick Start

```bash
# Build all services (release mode)
just docker-bake-all

# Build all services (debug mode)
just docker-bake-dev

# Build specific service
just docker-bake-bridge-api
```

## 📦 Build Commands

### All Services
```bash
just docker-bake-all              # Build all services (release)
just docker-bake-all-debug        # Build all services (debug)
just docker-bake-dev              # Build debug versions
just docker-bake-prod             # Build release versions
```

### Individual Services
```bash
# Bridge API
just docker-bake-bridge-api       # Build bridge-api (release)
just docker-bake-bridge-api-debug # Build bridge-api (debug)

# Schema Registry
just docker-bake-schema-registry       # Build schema-registry (release)
just docker-bake-schema-registry-debug # Build schema-registry (debug)

# Controller
just docker-bake-controller       # Build controller (release)
just docker-bake-controller-debug # Build controller (debug)
```

### Custom Builds
```bash
# Custom registry
just docker-bake-all-registry myregistry

# Custom tag
just docker-bake-all-tag v1.0.0

# Custom service with all options
just docker-bake-service bridge-api myregistry v1.0.0 debug

# Multi-platform build
just docker-bake-multi-platform
```

## 🏃‍♂️ Run Commands

```bash
# Run individual services
just docker-bake-run-bridge-api
just docker-bake-run-schema-registry
just docker-bake-run-controller
```

## 🔧 Utility Commands

```bash
# Show available images
just docker-bake-images

# Clean all images
just docker-bake-clean

# Build and push to registry
just docker-bake-push myregistry v1.0.0

# Show help
just docker-bake-help

# Show examples
just docker-bake-examples
```

## 📋 Common Workflows

### Development Workflow
```bash
# Build debug images for development
just docker-bake-dev

# Run a service for testing
just docker-bake-run-bridge-api
```

### Production Workflow
```bash
# Build release images
just docker-bake-prod

# Build with custom registry and tag
just docker-bake-push myregistry v1.0.0
```

### CI/CD Workflow
```bash
# Build for multiple platforms
just docker-bake-multi-platform

# Build and push to registry
just docker-bake-push ghcr.io/org/repo ${{ github.sha }}
```

## 🎯 Target Summary

| Command | Description | Build Type |
|---------|-------------|------------|
| `docker-bake-all` | All services | Release |
| `docker-bake-dev` | All services | Debug |
| `docker-bake-prod` | All services | Release |
| `docker-bake-bridge-api` | Bridge API only | Release |
| `docker-bake-schema-registry` | Schema Registry only | Release |
| `docker-bake-controller` | Controller only | Release |
| `docker-bake-multi-platform` | All services | Multi-platform |

## 🔍 Troubleshooting

```bash
# Check available images
just docker-bake-images

# Clean and rebuild
just docker-bake-clean
just docker-bake-all

# Show help for more options
just docker-bake-help
```

## 📚 Related Commands

- `just docker-build` - Build legacy Docker image
- `just docker-run` - Run legacy Docker container
- `just delta-*` - Delta Lake testing platform commands
