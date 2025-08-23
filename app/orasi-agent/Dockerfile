# Multi-stage build for Orasi Agent
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./
COPY ../../crates ./crates/

# Copy source code
COPY src ./src
COPY examples ./examples

# Build the application
RUN cargo build --release --bin orasi-agent

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r orasi && useradd -r -g orasi orasi

# Create directories
RUN mkdir -p /var/lib/orasi/agent /tmp/orasi/agent /etc/orasi/agent
RUN chown -R orasi:orasi /var/lib/orasi /tmp/orasi /etc/orasi

# Copy binary from builder
COPY --from=builder /app/target/release/orasi-agent /usr/local/bin/

# Copy configuration
COPY config/agent.toml /etc/orasi/agent/

# Set ownership
RUN chown orasi:orasi /usr/local/bin/orasi-agent /etc/orasi/agent/agent.toml

# Switch to non-root user
USER orasi

# Set working directory
WORKDIR /var/lib/orasi/agent

# Expose ports
EXPOSE 8082 8083 9092 8084

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8083/health/live || exit 1

# Default command
CMD ["orasi-agent"]
