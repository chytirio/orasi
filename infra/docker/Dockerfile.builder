# Multi-stage builder for Orasi OpenTelemetry Data Lake Bridge
# This stage compiles the entire workspace and makes binaries available

ARG RUST_VERSION=1.75-slim
ARG BUILD_TYPE=release

FROM rust:${RUST_VERSION} as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY app/ ./app/
COPY connectors/ ./connectors/
COPY infra/ ./infra/
COPY testing/ ./testing/
COPY examples/ ./examples/

# Create a dummy main.rs to build dependencies
RUN mkdir -p target/dummy && \
    echo 'fn main() {}' > target/dummy/main.rs

# Build dependencies first (this layer will be cached)
RUN cargo build --${BUILD_TYPE} --bin bridge-api --bin schema-registry --bin orasi-controller || true

# Remove dummy files and build actual binaries
RUN rm -rf target/dummy && \
    cargo build --${BUILD_TYPE} --bin bridge-api --bin schema-registry --bin orasi-controller

# Create a stage that just contains the compiled binaries
FROM scratch as binaries
COPY --from=builder /app/target/${BUILD_TYPE}/bridge-api /binaries/bridge-api
COPY --from=builder /app/target/${BUILD_TYPE}/schema-registry /binaries/schema-registry
COPY --from=builder /app/target/${BUILD_TYPE}/orasi-controller /binaries/controller

# Export binaries for use in other stages
FROM binaries as export
