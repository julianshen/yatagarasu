# Multi-stage Dockerfile for Yatagarasu S3 Proxy
# Optimized for minimal image size and security

# Stage 1: Build stage
# Use Rust 1.80+ for edition 2024 support
FROM rust:1-slim as builder

# Install build dependencies
# cmake and g++ needed for libz-ng-sys compilation
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    g++ \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary using cargo install (skips dev-dependencies and benches)
# This avoids issues with benchmark definitions in Cargo.toml
RUN cargo install --path . --root /app/install --locked

# Strip debug symbols to reduce binary size
RUN strip /app/install/bin/yatagarasu

# Stage 2: Runtime stage
FROM gcr.io/distroless/cc-debian12

# Metadata
LABEL maintainer="Yatagarasu Contributors"
LABEL description="High-performance S3 proxy with JWT authentication and HA bucket replication"
LABEL version="0.4.0"

# Copy binary from builder
COPY --from=builder /app/install/bin/yatagarasu /usr/local/bin/yatagarasu

# Copy example configuration for reference (can be overridden via volume mount)
COPY config.example.yaml /etc/yatagarasu/config.yaml.example

# Switch to non-root user (distroless has nonroot user with UID 65532)
USER 65532:65532

# Expose ports
# 8080: HTTP proxy server
# 9090: Prometheus metrics endpoint
EXPOSE 8080 9090

# Health check
# Note: Distroless has no shell/curl, so we check container health via external probes
# Use Kubernetes livenessProbe/readinessProbe or Docker HEALTHCHECK with HTTP checks
# For Docker standalone, this checks if the binary is executable
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/yatagarasu", "--version"]

# Default command
# Users should mount config: -v ./config.yaml:/etc/yatagarasu/config.yaml
ENTRYPOINT ["/usr/local/bin/yatagarasu"]
CMD ["--config", "/etc/yatagarasu/config.yaml"]
