#!/bin/bash

# Example usage script for Docker Bake configuration
# This demonstrates various ways to build and use the Orasi images

set -e

echo "=== Orasi Docker Bake Examples ==="
echo ""

# Example 1: Build all services in release mode
echo "Example 1: Build all services (release mode)"
echo "Command: ./infra/docker/build.sh"
echo "This will build:"
echo "  - orasi/bridge-api:latest"
echo "  - orasi/schema-registry:latest"
echo "  - orasi/controller:latest"
echo ""

# Example 2: Build specific service in debug mode
echo "Example 2: Build bridge-api in debug mode"
echo "Command: ./infra/docker/build.sh -t debug bridge-api"
echo "This will build:"
echo "  - orasi/bridge-api:debug"
echo ""

# Example 3: Build with custom registry and tag
echo "Example 3: Build with custom registry and tag"
echo "Command: ./infra/docker/build.sh -r myregistry --tag v1.0.0 prod"
echo "This will build:"
echo "  - myregistry/bridge-api:v1.0.0"
echo "  - myregistry/schema-registry:v1.0.0"
echo "  - myregistry/controller:v1.0.0"
echo ""

# Example 4: Build for multiple platforms
echo "Example 4: Build for multiple platforms"
echo "Command: ./infra/docker/build.sh -p 'linux/amd64,linux/arm64' all"
echo "This will build multi-platform images"
echo ""

# Example 5: Direct docker buildx bake usage
echo "Example 5: Direct docker buildx bake usage"
echo "Command: docker buildx bake bridge-api-release"
echo "This builds only the bridge-api release image"
echo ""

# Example 6: Build development images
echo "Example 6: Build development images"
echo "Command: ./infra/docker/build.sh dev"
echo "This will build:"
echo "  - orasi/bridge-api:debug"
echo "  - orasi/schema-registry:debug"
echo "  - orasi/controller:debug"
echo ""

# Example 7: Run a built image
echo "Example 7: Run a built image"
echo "Command: docker run -p 8080:8080 orasi/bridge-api:latest"
echo "This runs the bridge-api service on port 8080"
echo ""

# Example 8: Build and push to registry
echo "Example 8: Build and push to registry"
echo "Commands:"
echo "  ./infra/docker/build.sh -r myregistry --tag v1.0.0 all"
echo "  docker push myregistry/bridge-api:v1.0.0"
echo "  docker push myregistry/schema-registry:v1.0.0"
echo "  docker push myregistry/controller:v1.0.0"
echo ""

# Example 9: CI/CD usage
echo "Example 9: CI/CD usage"
echo "Command: docker buildx bake --push --set '*.tags=ghcr.io/org/repo/*:${{ github.sha }}' all"
echo "This builds and pushes to GitHub Container Registry"
echo ""

# Example 10: Debug build with verbose output
echo "Example 10: Debug build with verbose output"
echo "Command: docker buildx bake --progress=plain --set '*.args.BUILD_TYPE=debug' all"
echo "This shows detailed build output for debugging"
echo ""

echo "=== Environment Variables ==="
echo "You can also set these environment variables:"
echo "  BUILD_TYPE=debug|release"
echo "  REGISTRY=your-registry"
echo "  TAG=your-tag"
echo "  PLATFORMS=linux/amd64,linux/arm64"
echo ""

echo "=== Available Targets ==="
echo "Groups:"
echo "  all, dev, prod, bridge-api, schema-registry, controller"
echo ""
echo "Individual targets:"
echo "  bridge-api-debug, bridge-api-release"
echo "  schema-registry-debug, schema-registry-release"
echo "  controller-debug, controller-release"
echo ""

echo "=== Help ==="
echo "For more information, run:"
echo "  ./infra/docker/build.sh -h"
echo "  docker buildx bake --help"
