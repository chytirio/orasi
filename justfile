# Orasi (όραση) - High-performance Rust-based bridge for OpenTelemetry to data lakehouse
# Justfile for common development tasks

# Default recipe to show project status and key commands
default:
    @just status

# =============================================================================
# Core Development Commands
# =============================================================================

# Build the entire project
build:
    cargo build

# Build in release mode
build-release:
    cargo build --release

# Compile all tests
compile-all-tests:
    cargo test --workspace --no-run

# Run all tests
test:
    cargo test

# Run tests with verbose output
test-verbose:
    cargo test -- --nocapture

# Run tests for a specific crate
test-crate crate:
    cargo test -p {{crate}}

# Run integration tests
test-integration:
    cargo test --test integration

# Run benchmarks
bench:
    cargo bench

# Run benchmarks for a specific crate
bench-crate crate:
    cargo bench -p {{crate}}

# Check code formatting
fmt:
    cargo fmt

# Check code formatting and fix issues
fmt-fix:
    cargo fmt --all

# Run clippy linter
clippy:
    cargo clippy

# Run clippy with all warnings as errors
clippy-strict:
    cargo clippy -- -D warnings

# Run security audit
audit:
    cargo audit

# Generate test coverage report
coverage:
    cargo tarpaulin --out Html

# Clean build artifacts
clean:
    cargo clean

# Clean and rebuild
rebuild: clean build

# Install development dependencies
install-dev-deps:
    rustup component add rustfmt clippy
    cargo install cargo-tarpaulin
    cargo install cargo-husky

# Setup pre-commit hooks
setup-hooks:
    cargo install cargo-husky
    cargo husky install

# Generate documentation
doc:
    cargo doc --no-deps

# Generate documentation and open in browser
doc-open:
    cargo doc --no-deps --open

# Check documentation
doc-check:
    cargo doc --no-deps --document-private-items

# Run all checks (format, clippy, test, audit)
check: fmt clippy test audit

# Run all checks with coverage
check-full: fmt clippy test audit coverage

# Fix workspace resolver warning
fix-resolver:
    echo 'workspace.resolver = "2"' >> Cargo.toml

# Run full development workflow (format, lint, test, build)
dev-workflow: fmt clippy test build

# Run full CI workflow (format, lint, test, audit, build)
ci-workflow: fmt clippy test audit build-release

# Check if all dependencies are up to date
check-updates:
    cargo outdated

# Update dependencies
update-deps:
    cargo update

# Show project structure
tree:
    tree -I 'target|.git|node_modules' -a

# Show project size
size:
    du -sh . && echo "Target directory:" && du -sh target/

# Show Rust version and toolchain info
rust-info:
    rustc --version
    cargo --version
    rustup show

# Show workspace members
workspace-members:
    cargo metadata --format-version=version=1 | jq '.workspace_members'

# Show available binaries
binaries:
    cargo metadata --format-version=1 | jq -r '.workspace_members as $members | .packages[] | select(.name | IN($members[])) | select(.targets[] | .kind[] | contains("bin")) | "  - " + .name'

# Show help for this justfile
help:
    @just --list --verbose

# =============================================================================
# Modular Justfile Integration
# =============================================================================

# Show information about modular justfiles
modular-info:
    @echo "=== Orasi Modular Justfiles ==="
    @echo ""
    @echo "📁 Available justfiles:"
    @echo "  • Root:        ./justfile (this file)"
    @echo "  • Crates:      ./crates/justfile"
    @echo "  • Connectors:  ./connectors/justfile"
    @echo "  • Infrastructure: ./infra/justfile"
    @echo ""
    @echo "🚀 Usage:"
    @echo "  • just crates-info              # Show crates information"
    @echo "  • just connectors-info          # Show connectors information"
    @echo "  • just infra-info               # Show infrastructure information"
    @echo "  • just docker-bake-all          # Build all Docker services"
    @echo "  • just kind-setup               # Setup Kind cluster"
    @echo ""
    @echo "💡 Direct Usage:"
    @echo "  • cd crates && just build       # Build all crates"
    @echo "  • cd connectors && just test    # Test all connectors"
    @echo "  • cd infra && just docker-bake-all # Build Docker services"

# Show information about crates justfile
crates-info:
    cd crates && just info

# Show information about connectors justfile
connectors-info:
    cd connectors && just info

# Show information about infra justfile
infra-info:
    cd infra && just info

# Build all modules
build-all:
    cd crates && just build
    cd connectors && just build
    cd infra && just docker-bake-all

# Test all modules
test-all:
    cd crates && just test
    cd connectors && just test
    # Note: infra doesn't have tests, it's infrastructure-focused

# Run clippy on all modules
clippy-all:
    cd crates && just clippy
    cd connectors && just clippy
    # Note: infra doesn't have clippy, it's infrastructure-focused

# =============================================================================
# Application Commands
# =============================================================================

# Run schema registry (temporarily disabled)
run:
    cargo run --bin schema-registry

# Run schema registry with debug logging
run-debug:
    RUST_LOG=debug cargo run --bin schema-registry

# Run schema registry with specific config
run-config config:
    cargo run --bin schema-registry -- --config {{config}}

# Run test data generator
run-generator:
    cargo run -p test-data-generator

# Run test data generator with specific config
run-generator-config config:
    cargo run -p test-data-generator -- --config {{config}}

# Health check
health:
    curl http://localhost:8080/health

# Health check with detailed output
health-detailed:
    curl http://localhost:8080/health/detailed

# Metrics endpoint
metrics:
    curl http://localhost:9090/metrics

# Liveness probe
health-live:
    curl http://localhost:8080/health/live

# Readiness probe
health-ready:
    curl http://localhost:8080/health/ready

# Check all health endpoints
health-all: health health-detailed health-live health-ready metrics

# =============================================================================
# Example Commands
# =============================================================================

# Run basic usage example
example-basic:
    cargo run --example basic_usage

# Run auth example
example-auth:
    cargo run --example auth_example

# Run Delta Lake example
example-delta:
    cargo run --example delta_lake_example

# Run real Delta Lake example
example-delta-real:
    cargo run --example real_delta_lake_example

# Run Iceberg example
example-iceberg:
    cargo run --example iceberg_example

# Run Snowflake example
example-snowflake:
    cargo run --example snowflake_example

# Run Hudi example
example-hudi:
    cargo run --example hudi_example

# Run HTTP server example
example-http:
    cargo run --example http_server_example

# Run streaming processor example
example-streaming:
    cargo run --example streaming_processor_example

# Run query engine example
example-query:
    cargo run --example query_engine_example

# Run receiver example
example-receiver:
    cargo run --example receiver_example

# Run simple receiver example
example-receiver-simple:
    cargo run --example simple_receiver_example

# Run all examples
examples: example-basic example-auth example-delta example-iceberg example-snowflake example-hudi example-http example-streaming example-query example-receiver example-receiver-simple

# Run key examples (most useful for development)
examples-key: example-basic example-delta example-http example-streaming example-query

# =============================================================================
# Connector Testing Commands
# =============================================================================

# Run lakehouse connector tests using the Rust test runner
test-connectors:
    cargo run -p test-runner

# Run lakehouse connector tests with specific mode
test-connectors-mode mode:
    cargo run -p test-runner -- --mode {{mode}}

# Run lakehouse connector tests in parallel
test-connectors-parallel:
    cargo run -p test-runner -- --parallel

# Run lakehouse connector tests with coverage
test-connectors-coverage:
    cargo run -p test-runner -- --coverage

# Run lakehouse connector tests skipping external dependencies
test-connectors-skip-external:
    cargo run -p test-runner -- --skip-external

# List available connectors
test-connectors-list:
    cargo run -p test-runner -- list

# Show test configuration
test-connectors-config:
    cargo run -p test-runner -- config

# Run tests for specific connectors
test-connectors-specific connectors:
    cargo run -p test-runner -- {{connectors}}

# =============================================================================
# Docker Commands
# =============================================================================

# Check if Docker is available and build image
docker-build:
    docker build -f infra/docker/Dockerfile -t orasi .

# Run Docker container
docker-run:
    docker run -f infra/docker/Dockerfile -p 4317:4317 -p 4318:4318 -p 9090:9090 -p 8080:8080 orasi

# Run Docker container with config volume
docker-run-config:
    docker run  -f infra/docker/Dockerfile -p 4317:4317 -p 4318:4318 -p 9090:9090 -p 8080:8080 -v ./config:/config orasi

# =============================================================================
# Docker Bake Targets
# =============================================================================

# Build all services (release mode)
docker-bake-all:
    ./infra/docker/build.sh

# Build all services (debug mode)
docker-bake-all-debug:
    ./infra/docker/build.sh -t debug all

# Build all services (release mode) with custom registry
docker-bake-all-registry registry:
    ./infra/docker/build.sh -r {{registry}} all

# Build all services with custom tag
docker-bake-all-tag tag:
    ./infra/docker/build.sh --tag {{tag}} all

# Build development images (debug mode)
docker-bake-dev:
    ./infra/docker/build.sh -t debug dev

# Build production images (release mode)
docker-bake-prod:
    ./infra/docker/build.sh prod

# Build bridge-api service (release mode)
docker-bake-bridge-api:
    ./infra/docker/build.sh bridge-api

# Build bridge-api service (debug mode)
docker-bake-bridge-api-debug:
    ./infra/docker/build.sh -t debug bridge-api

# Build schema-registry service (release mode)
docker-bake-schema-registry:
    ./infra/docker/build.sh schema-registry

# Build schema-registry service (debug mode)
docker-bake-schema-registry-debug:
    ./infra/docker/build.sh -t debug schema-registry

# Build controller service (release mode)
docker-bake-controller:
    ./infra/docker/build.sh controller

# Build controller service (debug mode)
docker-bake-controller-debug:
    ./infra/docker/build.sh -t debug controller

# Build specific service with custom settings
docker-bake-service service registry="orasi" tag="latest" build_type="release":
    ./infra/docker/build.sh -r {{registry}} --tag {{tag}} -t {{build_type}} {{service}}

# Build for multiple platforms
docker-bake-multi-platform:
    ./infra/docker/build.sh -p "linux/amd64,linux/arm64" all

# Build and push to registry
docker-bake-push registry tag="latest":
    ./infra/docker/build.sh -r {{registry}} --tag {{tag}} all
    docker push {{registry}}/bridge-api:{{tag}}
    docker push {{registry}}/schema-registry:{{tag}}
    docker push {{registry}}/controller:{{tag}}

# Show available Docker images
docker-bake-images:
    docker images | grep "orasi" || echo "No orasi images found"

# Clean Docker images
docker-bake-clean:
    docker rmi $(docker images | grep "orasi" | awk '{print $3}') 2>/dev/null || echo "No orasi images to clean"

# Run bridge-api container
docker-bake-run-bridge-api:
    docker run -p 8080:8080 orasi/bridge-api:latest

# Run schema-registry container
docker-bake-run-schema-registry:
    docker run -p 8080:8080 orasi/schema-registry:latest

# Run controller container
docker-bake-run-controller:
    docker run -p 8080:8080 orasi/controller:latest

# Show Docker Bake help
docker-bake-help:
    ./infra/docker/build.sh -h

# Show Docker Bake examples
docker-bake-examples:
    ./infra/example-usage.sh

# =============================================================================
# Docker Compose with Bake Integration
# =============================================================================

# Start all services with baked images (release mode)
compose-bake-up:
    docker-compose -f infra/docker/docker-compose.bake.yml up -d

# Start all services with baked images (debug mode)
compose-bake-up-dev:
    docker-compose -f infra/docker/docker-compose.bake.dev.yml up -d

# Start all services with baked images and logs
compose-bake-up-logs:
    docker-compose -f infra/docker/docker-compose.bake.yml up

# Start all services with baked images and logs (debug mode)
compose-bake-up-logs-dev:
    docker-compose -f infra/docker/docker-compose.bake.dev.yml up

# Stop all services
compose-bake-down:
    docker-compose -f infra/docker/docker-compose.bake.yml down

# Stop all services (debug mode)
compose-bake-down-dev:
    docker-compose -f infra/docker/docker-compose.bake.dev.yml down

# Stop all services and remove volumes
compose-bake-down-clean:
    docker-compose -f infra/docker/docker-compose.bake.yml down -v

# Stop all services and remove volumes (debug mode)
compose-bake-down-clean-dev:
    docker-compose -f infra/docker/docker-compose.bake.dev.yml down -v

# Restart all services
compose-bake-restart:
    docker-compose -f infra/docker/docker-compose.bake.yml restart

# Restart all services (debug mode)
compose-bake-restart-dev:
    docker-compose -f infra/docker/docker-compose.bake.dev.yml restart

# Show status of all services
compose-bake-status:
    docker-compose -f infra/docker/docker-compose.bake.yml ps

# Show status of all services (debug mode)
compose-bake-status-dev:
    docker-compose -f infra/docker/docker-compose.bake.dev.yml ps

# Show logs for all services
compose-bake-logs:
    docker-compose -f infra/docker/docker-compose.bake.yml logs

# Show logs for all services (debug mode)
compose-bake-logs-dev:
    docker-compose -f infra/docker/docker-compose.bake.dev.yml logs

# Show logs for specific service
compose-bake-logs-service service:
    docker-compose -f infra/docker/docker-compose.bake.yml logs {{service}}

# Show logs for specific service (debug mode)
compose-bake-logs-service-dev service:
    docker-compose -f infra/docker/docker-compose.bake.dev.yml logs {{service}}

# Follow logs for all services
compose-bake-logs-follow:
    docker-compose -f infra/docker/docker-compose.bake.yml logs -f

# Follow logs for all services (debug mode)
compose-bake-logs-follow-dev:
    docker-compose -f infra/docker/docker-compose.bake.dev.yml logs -f

# Follow logs for specific service
compose-bake-logs-follow-service service:
    docker-compose -f infra/docker/docker-compose.bake.yml logs -f {{service}}

# Follow logs for specific service (debug mode)
compose-bake-logs-follow-service-dev service:
    docker-compose -f infra/docker/docker-compose.bake.dev.yml logs -f {{service}}

# Build and start all services
compose-bake-build-up:
    just docker-bake-all
    just compose-bake-up

# Build and start all services (debug mode)
compose-bake-build-up-dev:
    just docker-bake-dev
    just compose-bake-up-dev

# Build, push, and start all services
compose-bake-build-push-up registry tag="latest":
    just docker-bake-push {{registry}} {{tag}}
    BUILD_TYPE=release docker-compose -f infra/docker/docker-compose.bake.yml up -d

# Build, push, and start all services (debug mode)
compose-bake-build-push-up-dev registry tag="debug":
    just docker-bake-push {{registry}} {{tag}}
    docker-compose -f infra/docker/docker-compose.bake.dev.yml up -d

# =============================================================================
# Delta Lake Testing Platform
# =============================================================================

# Start the Delta Lake testing platform
delta-up:
    docker-compose -f infra/docker/docker-compose.yml up -d

# Start the Delta Lake testing platform with logs
delta-up-logs:
    docker-compose -f infra/docker/docker-compose.yml up

# Stop the Delta Lake testing platform
delta-down:
    docker-compose -f infra/docker/docker-compose.yml down

# Stop the Delta Lake testing platform and remove volumes
delta-down-clean:
    docker-compose -f infra/docker/docker-compose.yml down -v

# Restart the Delta Lake testing platform
delta-restart:
    docker-compose -f infra/docker/docker-compose.yml restart

# Show status of Delta Lake testing platform
delta-status:
    docker-compose -f infra/docker/docker-compose.yml ps

# Show logs for all Delta Lake services
delta-logs:
    docker-compose -f infra/docker/docker-compose.yml logs

# Show logs for specific Delta Lake service
delta-logs-service service:
    docker-compose -f infra/docker/docker-compose.yml logs {{service}}

# Follow logs for all Delta Lake services
delta-logs-follow:
    docker-compose -f infra/docker/docker-compose.yml logs -f

# Follow logs for specific Delta Lake service
delta-logs-follow-service service:
    docker-compose -f infra/docker/docker-compose.yml logs -f {{service}}

# Setup the Delta Lake testing platform (full initialization)
delta-setup:
    ./scripts/setup-testing-platform.sh

# Run Delta Lake tests
delta-test:
    ./scripts/test-delta-lake.sh

# Initialize MinIO buckets
delta-init-buckets:
    docker exec orasi-minio mc alias set minio http://localhost:9000 minioadmin minioadmin || true
    docker exec orasi-minio mc mb minio/test-bucket || true
    docker exec orasi-minio mc mb minio/spark-logs || true
    docker exec orasi-minio mc mb minio/telemetry-data || true

# Test MinIO connectivity
delta-test-minio:
    curl -f http://localhost:9000/minio/health/live

# Test Spark connectivity
delta-test-spark:
    curl -f http://localhost:8080

# Test all Delta Lake services
delta-test-all: delta-test-minio delta-test-spark

# Open MinIO console in browser
delta-minio:
    open http://localhost:9001

# Open Spark Master UI in browser
delta-spark:
    open http://localhost:8080

# Open Spark Worker UI in browser
delta-spark-worker:
    open http://localhost:8081

# Open Spark History Server in browser
delta-spark-history:
    open http://localhost:18080

# Open Jupyter Notebook in browser
delta-jupyter:
    open http://localhost:8888

# Open all Delta Lake service UIs in browser
delta-open-all: delta-minio delta-spark delta-spark-worker delta-spark-history delta-jupyter

# Run Delta Lake test container
delta-run-test:
    docker-compose run --rm delta-test

# Run custom command in Delta Lake test container
delta-run-test-cmd cmd:
    docker-compose run --rm delta-test {{cmd}}

# Execute Python script in Delta Lake test container
delta-run-python script:
    docker-compose run --rm delta-test python {{script}}

# Execute shell command in Delta Lake test container
delta-run-shell cmd:
    docker-compose run --rm delta-test sh -c "{{cmd}}"

# Clean Delta Lake data
delta-clean-data:
    rm -rf data/*
    rm -rf notebooks/.ipynb_checkpoints

# Clean Delta Lake volumes
delta-clean-volumes:
    docker volume rm orasi_minio_data orasi_spark_data || true

# Full cleanup of Delta Lake testing platform
delta-clean-all: delta-down-clean delta-clean-data delta-clean-volumes

# Show Delta Lake testing platform information
delta-info:
    @echo "=== Delta Lake Testing Platform ==="
    @echo "🐳 Services:"
    @docker-compose ps 2>/dev/null || echo "  Services not running"
    @echo ""
    @echo "🌐 Service URLs:"
    @echo "  • MinIO Console:     http://localhost:9001 (minioadmin/minioadmin)"
    @echo "  • Spark Master:      http://localhost:8080"
    @echo "  • Spark Worker:      http://localhost:8081"
    @echo "  • Spark History:     http://localhost:18080"
    @echo "  • Jupyter Notebook:  http://localhost:8888"
    @echo ""
    @echo "📁 Directories:"
    @echo "  • Data:              ./data"
    @echo "  • Notebooks:         ./notebooks"
    @echo "  • Configuration:     ./config"
    @echo ""
    @echo "🔧 Key Commands:"
    @echo "  • just delta-setup   # Full setup and initialization"
    @echo "  • just delta-test    # Run tests"
    @echo "  • just delta-up      # Start services"
    @echo "  • just delta-down    # Stop services"
    @echo "  • just delta-logs    # View logs"
    @echo "  • just delta-open-all # Open all UIs"

# Quick start Delta Lake testing platform
delta-quickstart: delta-setup delta-test delta-info

# =============================================================================
# Kind Cluster for Local Testing
# =============================================================================

# Setup Kind cluster and deploy Orasi services
kind-setup:
    cd infra/.dev/kind && ./setup.sh

# Teardown Kind cluster and clean up resources
kind-teardown:
    cd infra/.dev/kind && ./teardown.sh

# Teardown Kind cluster with full cleanup (cluster + Docker + images)
kind-teardown-full:
    cd infra/.dev/kind && ./teardown.sh full

# Test Kind cluster health and service availability
kind-test:
    cd infra/.dev/kind && ./test-cluster.sh

# Show Kind cluster status
kind-status:
    kubectl get nodes
    kubectl get pods -n orasi
    kubectl get services -n orasi

# Show Kind cluster logs
kind-logs:
    kubectl logs -f -l app=bridge-api -n orasi & \
    kubectl logs -f -l app=schema-registry -n orasi & \
    kubectl logs -f -l app=web-ui -n orasi & \
    wait

# Show logs for specific service
kind-logs-service service:
    kubectl logs -f deployment/{{service}} -n orasi

# Show logs for bridge-api
kind-logs-bridge-api:
    kubectl logs -f deployment/bridge-api -n orasi

# Show logs for schema-registry
kind-logs-schema-registry:
    kubectl logs -f deployment/schema-registry -n orasi

# Show logs for web-ui
kind-logs-web-ui:
    kubectl logs -f deployment/web-ui -n orasi

# Restart all Orasi services
kind-restart:
    kubectl rollout restart deployment/bridge-api -n orasi
    kubectl rollout restart deployment/schema-registry -n orasi
    kubectl rollout restart deployment/web-ui -n orasi

# Restart specific service
kind-restart-service service:
    kubectl rollout restart deployment/{{service}} -n orasi

# Scale services
kind-scale service replicas:
    kubectl scale deployment/{{service}} --replicas={{replicas}} -n orasi

# Scale all services
kind-scale-all replicas:
    kubectl scale deployment/bridge-api --replicas={{replicas}} -n orasi
    kubectl scale deployment/schema-registry --replicas={{replicas}} -n orasi
    kubectl scale deployment/web-ui --replicas={{replicas}} -n orasi

# Port forward services for local access
kind-port-forward:
    kubectl port-forward service/bridge-api 30080:8080 -n orasi & \
    kubectl port-forward service/schema-registry 30081:8080 -n orasi & \
    kubectl port-forward service/web-ui 30082:8080 -n orasi & \
    echo "Services available at:"
    echo "  Bridge API: http://localhost:30080"
    echo "  Schema Registry: http://localhost:30081"
    echo "  Web UI: http://localhost:30082"
    echo "Press Ctrl+C to stop port forwarding"
    wait

# Port forward specific service
kind-port-forward-service service port:
    kubectl port-forward service/{{service}} {{port}}:8080 -n orasi

# Port forward bridge-api
kind-port-forward-bridge-api:
    kubectl port-forward service/bridge-api 30080:8080 -n orasi

# Port forward schema-registry
kind-port-forward-schema-registry:
    kubectl port-forward service/schema-registry 30081:8080 -n orasi

# Port forward web-ui
kind-port-forward-web-ui:
    kubectl port-forward service/web-ui 30082:8080 -n orasi

# Test service health endpoints
kind-health:
    curl -f http://localhost:30080/health/live || echo "Bridge API health check failed"
    curl -f http://localhost:30081/health/live || echo "Schema Registry health check failed"
    curl -f http://localhost:30082/health || echo "Web UI health check failed"

# Test bridge-api health
kind-health-bridge-api:
    curl -f http://localhost:30080/health/live

# Test schema-registry health
kind-health-schema-registry:
    curl -f http://localhost:30081/health/live

# Test web-ui health
kind-health-web-ui:
    curl -f http://localhost:30082/health

# Open service URLs in browser
kind-open:
    open http://localhost:30080 || echo "Could not open Bridge API"
    open http://localhost:30081 || echo "Could not open Schema Registry"
    open http://localhost:30082 || echo "Could not open Web UI"

# Open bridge-api in browser
kind-open-bridge-api:
    open http://localhost:30080

# Open schema-registry in browser
kind-open-schema-registry:
    open http://localhost:30081

# Open web-ui in browser
kind-open-web-ui:
    open http://localhost:30082

# Build and load Docker images for Kind cluster
kind-build-images:
    just docker-bake-all
    kind load docker-image orasi/bridge-api:latest --name orasi-test
    kind load docker-image orasi/schema-registry:latest --name orasi-test
    kind load docker-image orasi/web:latest --name orasi-test

# Build and load specific image for Kind cluster
kind-build-image service:
    just docker-bake-{{service}}
    kind load docker-image orasi/{{service}}:latest --name orasi-test

# Deploy Orasi services to Kind cluster
kind-deploy:
    kubectl apply -f infra/.dev/kind/k8s/ -n orasi

# Delete Orasi services from Kind cluster
kind-delete:
    kubectl delete -f infra/.dev/kind/k8s/ -n orasi

# Show Kind cluster information
kind-info:
    @echo "=== Kind Cluster Information ==="
    @echo "🐳 Cluster:"
    @kind get clusters 2>/dev/null || echo "  No clusters found"
    @echo ""
    @echo "🌐 Service URLs (if port forwarding is active):"
    @echo "  • Bridge API:      http://localhost:30080"
    @echo "  • Schema Registry: http://localhost:30081"
    @echo "  • Web UI:          http://localhost:30082"
    @echo ""
    @echo "🔧 Key Commands:"
    @echo "  • just kind-setup   # Setup cluster and deploy services"
    @echo "  • just kind-test    # Test cluster health"
    @echo "  • just kind-status  # Show cluster status"
    @echo "  • just kind-logs    # View service logs"
    @echo "  • just kind-teardown # Cleanup cluster"
    @echo "  • just kind-port-forward # Port forward services"
    @echo "  • just kind-open    # Open services in browser"

# Quick start Kind cluster
kind-quickstart: kind-setup kind-test kind-info

# =============================================================================
# Project Status and Information
# =============================================================================

# Show project status
status:
    @echo "=== Orasi Project Status ==="
    @echo "📦 Available binaries:"
    @cargo metadata --format-version=1 | jq -r '.workspace_members as $members | .packages[] | select(.name | IN($members[])) | select(.targets[] | .kind[] | contains("bin")) | "  - " + .name' 2>/dev/null || echo "  - schema-registry (temporarily disabled)"
    @echo ""
    @echo "🚀 Key examples to run:"
    @echo "  - just example-basic     # Basic usage example"
    @echo "  - just example-delta     # Delta Lake integration"
    @echo "  - just example-http      # HTTP server example"
    @echo "  - just example-streaming # Streaming processor"
    @echo "  - just example-query     # Query engine"
    @echo ""
    @echo "🔧 Development commands:"
    @echo "  - just dev-workflow      # Format, lint, test, build"
    @echo "  - just examples-key      # Run key examples"
    @echo "  - just fix-resolver      # Fix workspace resolver warning"
    @echo ""
    @echo "🐳 Docker Bake commands:"
    @echo "  - just docker-bake-all   # Build all services (release)"
    @echo "  - just docker-bake-dev   # Build all services (debug)"
    @echo "  - just docker-bake-prod  # Build all services (release)"
    @echo "  - just docker-bake-help  # Show Docker Bake help"
    @echo ""
    @echo "🐳 Docker Compose with Bake:"
    @echo "  - just compose-bake-up   # Start all services (release)"
    @echo "  - just compose-bake-up-dev # Start all services (debug)"
    @echo "  - just compose-bake-build-up # Build and start (release)"
    @echo "  - just compose-bake-build-up-dev # Build and start (debug)"
    @echo ""
    @echo "🐳 Delta Lake Testing Platform:"
    @echo "  - just delta-quickstart  # Quick setup and test"
    @echo "  - just delta-setup       # Full setup and initialization"
    @echo "  - just delta-test        # Run Delta Lake tests"
    @echo "  - just delta-info        # Show platform information"
    @echo "  - just delta-open-all    # Open all service UIs"
    @echo ""
    @echo "🐳 Kind Cluster for Local Testing:"
    @echo "  - just kind-quickstart   # Quick setup and test"
    @echo "  - just kind-setup        # Setup cluster and deploy services"
    @echo "  - just kind-test         # Test cluster health"
    @echo "  - just kind-status       # Show cluster status"
    @echo "  - just kind-teardown     # Cleanup cluster"
    @echo "  - just kind-port-forward # Port forward services"
    @echo "  - just kind-open         # Open services in browser"
