#!/bin/bash

# Build script for Orasi OpenTelemetry Data Lake Bridge using Docker Bake
# This script demonstrates how to build individual services or all services

set -e

# Default values
BUILD_TYPE=${BUILD_TYPE:-"release"}
REGISTRY=${REGISTRY:-"orasi"}
TAG=${TAG:-"latest"}
PLATFORMS=${PLATFORMS:-"linux/amd64"}

# Function to show usage
show_usage() {
    echo "Usage: $0 [OPTIONS] [TARGETS...]"
    echo ""
    echo "Options:"
    echo "  -h, --help          Show this help message"
    echo "  -t, --type TYPE     Build type (debug|release) [default: release]"
    echo "  -r, --registry REG  Docker registry [default: orasi]"
    echo "  --tag TAG           Image tag [default: latest]"
    echo "  -p, --platforms P   Target platforms [default: linux/amd64]"
    echo ""
    echo "Targets:"
    echo "  all                 Build all services (default)"
    echo "  bridge-api          Build bridge-api service"
    echo "  schema-registry     Build schema-registry service"
    echo "  controller          Build controller service"
    echo "  dev                 Build debug versions of all services"
    echo "  prod                Build release versions of all services"
    echo ""
    echo "Examples:"
    echo "  $0                                    # Build all release images"
    echo "  $0 -t debug bridge-api               # Build debug bridge-api"
    echo "  $0 -r myregistry -t release dev      # Build debug images with custom registry"
    echo "  $0 --tag v1.0.0 prod                 # Build release images with specific tag"
}

# Parse command line arguments
TARGETS=()
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_usage
            exit 0
            ;;
        -t|--type)
            BUILD_TYPE="$2"
            shift 2
            ;;
        -r|--registry)
            REGISTRY="$2"
            shift 2
            ;;
        --tag)
            TAG="$2"
            shift 2
            ;;
        -p|--platforms)
            PLATFORMS="$2"
            shift 2
            ;;
        -*)
            echo "Unknown option: $1"
            show_usage
            exit 1
            ;;
        *)
            TARGETS+=("$1")
            shift
            ;;
    esac
done

# Default to "all" if no targets specified
if [ ${#TARGETS[@]} -eq 0 ]; then
    TARGETS=("all")
fi

# Validate build type
if [[ "$BUILD_TYPE" != "debug" && "$BUILD_TYPE" != "release" ]]; then
    echo "Error: BUILD_TYPE must be 'debug' or 'release'"
    exit 1
fi

# Get the absolute path to the project root directory
PROJECT_ROOT="$(pwd)"

echo "Building Orasi OpenTelemetry Data Lake Bridge"
echo "Build Type: $BUILD_TYPE"
echo "Registry: $REGISTRY"
echo "Tag: $TAG"
echo "Platforms: $PLATFORMS"
echo "Targets: ${TARGETS[*]}"
echo "Build Path: $PROJECT_ROOT"
echo ""

cd "$PROJECT_ROOT"
# Build each target
for target in "${TARGETS[@]}"; do
    echo "Building target: $target"
    
    case $target in
        all)
            docker buildx bake \
                --set "*.args.BUILD_TYPE=$BUILD_TYPE" \
                all
            ;;
        bridge-api)
            docker buildx bake \
                --set "*.args.BUILD_TYPE=$BUILD_TYPE" \
                bridge-api
            ;;
        schema-registry)
            docker buildx bake \
                --set "*.args.BUILD_TYPE=$BUILD_TYPE" \
                schema-registry
            ;;
        controller)
            docker buildx bake \
                --set "*.args.BUILD_TYPE=$BUILD_TYPE" \
                controller
            ;;
        dev)
            docker buildx bake dev
            ;;
        prod)
            docker buildx bake prod
            ;;
        *)
            echo "Error: Unknown target '$target'"
            show_usage
            exit 1
            ;;
    esac
    
    echo "Completed building target: $target"
    echo ""
done

echo "Build completed successfully!"
echo ""
echo "Available images:"
docker images | grep "$REGISTRY" || echo "No images found for registry: $REGISTRY"
