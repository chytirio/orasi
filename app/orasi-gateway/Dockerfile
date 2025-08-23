# Multi-stage build for Orasi Gateway
FROM rust:1.75-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    pkgconfig \
    && rm -rf /var/cache/apk/*

# Set working directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./
COPY ../../Cargo.toml ../../Cargo.lock ../../
COPY ../../crates/bridge-core/Cargo.toml ../../crates/bridge-core/
COPY ../orasi-bridge/Cargo.toml ../orasi-bridge/

# Create dummy source files for dependencies
RUN mkdir -p src ../../crates/bridge-core/src ../orasi-bridge/src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn lib() {}" > ../../crates/bridge-core/src/lib.rs && \
    echo "pub fn lib() {}" > ../orasi-bridge/src/lib.rs

# Build dependencies
RUN cargo build --release --bin orasi-gateway

# Remove dummy files and copy real source
RUN rm -rf src ../../crates/bridge-core/src ../orasi-bridge/src
COPY . .

# Build the application
RUN cargo build --release --bin orasi-gateway

# Runtime stage
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    openssl \
    && rm -rf /var/cache/apk/*

# Create non-root user
RUN addgroup -g 1001 -S orasi && \
    adduser -u 1001 -S orasi -G orasi

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/orasi-gateway /app/orasi-gateway

# Copy configuration
COPY ../../config/gateway.toml /app/config/gateway.toml

# Create necessary directories
RUN mkdir -p /app/logs /app/certs && \
    chown -R orasi:orasi /app

# Switch to non-root user
USER orasi

# Expose ports
EXPOSE 8080 8081 9090 8082

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8081/health/live || exit 1

# Default command
CMD ["/app/orasi-gateway"]
