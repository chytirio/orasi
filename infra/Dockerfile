# Multi-stage build for OpenTelemetry Data Lake Bridge
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build the project
RUN cargo build --release --bin bridge

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r bridge && useradd -r -g bridge bridge

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/bridge /app/bridge

# Copy configuration files
COPY config/ ./config/

# Create necessary directories
RUN mkdir -p /app/logs /app/data && \
    chown -R bridge:bridge /app

# Switch to non-root user
USER bridge

# Expose ports
EXPOSE 4317 4318 9090 8080 8081

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health/live || exit 1

# Default command
CMD ["/app/bridge", "--config", "/app/config/bridge.toml"]
