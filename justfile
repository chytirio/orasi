# Orasi (όραση) - High-performance Rust-based bridge for OpenTelemetry to data lakehouse
# Justfile for common development tasks

# Default recipe to show project status and key commands
default:
    @just status

# Build the entire project
build:
    cargo build

# Build in release mode
build-release:
    cargo build --release

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

# Run Kafka example (disabled)
# example-kafka:
#     cargo run --example kafka_example

# Run Hudi example
example-hudi:
    cargo run --example hudi_example

# Run S3 Parquet example
example-s3-parquet:
    cargo run --example s3_parquet_example

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

# Run Kafka connector example (disabled)
# example-kafka-connector:
#     cargo run --example kafka_connector_example

# Run all examples
examples: example-basic example-auth example-delta example-iceberg example-snowflake example-hudi example-s3-parquet example-http example-streaming example-query example-receiver example-receiver-simple

# Run key examples (most useful for development)
examples-key: example-basic example-delta example-http example-streaming example-query

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

# Check if Docker is available and build image
docker-build:
    docker build -f infra/Dockerfile -t orasi .

# Run Docker container
docker-run:
    docker run -f infra/Dockerfile -p 4317:4317 -p 4318:4318 -p 9090:9090 -p 8080:8080 orasi

# Run Docker container with config volume
docker-run-config:
    docker run  -f infra/Dockerfile -p 4317:4317 -p 4318:4318 -p 9090:9090 -p 8080:8080 -v ./config:/config orasi

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
    @echo "🐳 Delta Lake Testing Platform:"
    @echo "  - just delta-quickstart  # Quick setup and test"
    @echo "  - just delta-setup       # Full setup and initialization"
    @echo "  - just delta-test        # Run Delta Lake tests"
    @echo "  - just delta-info        # Show platform information"
    @echo "  - just delta-open-all    # Open all service UIs"

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

# Show help for this justfile
help:
    @just --list --verbose

# =============================================================================
# Delta Lake Testing Platform
# =============================================================================

# Start the Delta Lake testing platform
delta-up:
    docker-compose -f infra/docker-compose.yml up -d

# Start the Delta Lake testing platform with logs
delta-up-logs:
    docker-compose -f infra/docker-compose.yml up

# Stop the Delta Lake testing platform
delta-down:
    docker-compose -f infra/docker-compose.yml down

# Stop the Delta Lake testing platform and remove volumes
delta-down-clean:
    docker-compose -f infra/docker-compose.yml down -v

# Restart the Delta Lake testing platform
delta-restart:
    docker-compose -f infra/docker-compose.yml restart

# Show status of Delta Lake testing platform
delta-status:
    docker-compose -f infra/docker-compose.yml ps

# Show logs for all Delta Lake services
delta-logs:
    docker-compose -f infra/docker-compose.yml logs

# Show logs for specific Delta Lake service
delta-logs-service service:
    docker-compose -f infra/docker-compose.yml logs {{service}}

# Follow logs for all Delta Lake services
delta-logs-follow:
    docker-compose -f infra/docker-compose.yml logs -f

# Follow logs for specific Delta Lake service
delta-logs-follow-service service:
    docker-compose -f infra/docker-compose.yml logs -f {{service}}

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
