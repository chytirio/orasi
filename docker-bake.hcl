// Docker Bake configuration for Orasi OpenTelemetry Data Lake Bridge
// Supports building individual images for bridge-api, schema-registry, and controller
// in both debug and release modes

// Variables that can be overridden
variable "BUILD_TYPE" {
  default = "release"
}

variable "RUST_VERSION" {
  default = "latest"
}

variable "REGISTRY" {
  default = "orasi"
}

variable "TAG" {
  default = "latest"
}



// Base builder stage
group "builder" {
  targets = ["workspace-builder"]
}

target "workspace-builder" {
  dockerfile = "infra/docker/Dockerfile.builder"
  context = "."
  args = {
    RUST_VERSION = RUST_VERSION
    BUILD_TYPE = BUILD_TYPE
  }
  tags = ["${REGISTRY}/workspace-builder:${TAG}"]
  cache-from = ["type=gha"]
  cache-to = ["type=gha,mode=max"]
}

// Bridge API targets
group "bridge-api" {
  targets = ["bridge-api-debug", "bridge-api-release"]
}

target "bridge-api-debug" {
  dockerfile = "infra/docker/Dockerfile.service"
  context = "."
  args = {
    BINARY_NAME = "bridge-api"
    BUILD_TYPE = "debug"
    RUST_VERSION = RUST_VERSION
  }
  tags = ["${REGISTRY}/bridge-api:debug", "${REGISTRY}/bridge-api:debug-${TAG}"]
  cache-from = ["type=gha"]
  cache-to = ["type=gha,mode=max"]
}

target "bridge-api-release" {
  dockerfile = "infra/docker/Dockerfile.service"
  context = "."
  args = {
    BINARY_NAME = "bridge-api"
    BUILD_TYPE = "release"
    RUST_VERSION = RUST_VERSION
  }
  tags = ["${REGISTRY}/bridge-api:release", "${REGISTRY}/bridge-api:${TAG}"]
  cache-from = ["type=gha"]
  cache-to = ["type=gha,mode=max"]
}

// Schema Registry targets
group "schema-registry" {
  targets = ["schema-registry-debug", "schema-registry-release"]
}

target "schema-registry-debug" {
  dockerfile = "infra/docker/Dockerfile.service"
  context = "."
  args = {
    BINARY_NAME = "schema-registry"
    BUILD_TYPE = "debug"
    RUST_VERSION = RUST_VERSION
  }
  tags = ["${REGISTRY}/schema-registry:debug", "${REGISTRY}/schema-registry:debug-${TAG}"]
  cache-from = ["type=gha"]
  cache-to = ["type=gha,mode=max"]
}

target "schema-registry-release" {
  dockerfile = "infra/docker/Dockerfile.service"
  context = "."
  args = {
    BINARY_NAME = "schema-registry"
    BUILD_TYPE = "release"
    RUST_VERSION = RUST_VERSION
  }
  tags = ["${REGISTRY}/schema-registry:release", "${REGISTRY}/schema-registry:${TAG}"]
  cache-from = ["type=gha"]
  cache-to = ["type=gha,mode=max"]
}

// Controller targets
group "controller" {
  targets = ["controller-debug", "controller-release"]
}

target "controller-debug" {
  dockerfile = "infra/docker/Dockerfile.service"
  context = "."
  args = {
    BINARY_NAME = "controller"
    BUILD_TYPE = "debug"
    RUST_VERSION = RUST_VERSION
  }
  tags = ["${REGISTRY}/controller:debug", "${REGISTRY}/controller:debug-${TAG}"]
  cache-from = ["type=gha"]
  cache-to = ["type=gha,mode=max"]
}

target "controller-release" {
  dockerfile = "infra/docker/Dockerfile.service"
  context = "."
  args = {
    BINARY_NAME = "controller"
    BUILD_TYPE = "release"
    RUST_VERSION = RUST_VERSION
  }
  tags = ["${REGISTRY}/controller:release", "${REGISTRY}/controller:${TAG}"]
  cache-from = ["type=gha"]
  cache-to = ["type=gha,mode=max"]
}

// Base runtime configuration
target "base-runtime" {
  dockerfile = "infra/docker/Dockerfile.runtime"
  context = "."
  args = {
    RUST_VERSION = RUST_VERSION
  }
}

// All targets group
group "all" {
  targets = [
    "bridge-api-debug",
    "bridge-api-release", 
    "schema-registry-debug",
    "schema-registry-release",
    "controller-debug",
    "controller-release"
  ]
}

// Development targets (debug builds)
group "dev" {
  targets = [
    "bridge-api-debug",
    "schema-registry-debug", 
    "controller-debug"
  ]
}

// Production targets (release builds)
group "prod" {
  targets = [
    "bridge-api-release",
    "schema-registry-release",
    "controller-release"
  ]
}
