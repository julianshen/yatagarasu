# Multi-stage Dockerfile for Yatagarasu S3 Proxy
# Optimized for minimal image size and security

# Stage 1: Build stage
FROM rust:1.70-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY tests ./tests

# Build release binary
RUN cargo build --release --bin yatagarasu

# Strip debug symbols to reduce binary size
RUN strip /app/target/release/yatagarasu

# Stage 2: Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash yatagarasu

# Create directories
RUN mkdir -p /etc/yatagarasu /var/log/yatagarasu && \
    chown -R yatagarasu:yatagarasu /etc/yatagarasu /var/log/yatagarasu

# Copy binary from builder
COPY --from=builder /app/target/release/yatagarasu /usr/local/bin/yatagarasu
RUN chmod +x /usr/local/bin/yatagarasu

# Copy example configuration (can be overridden via volume mount)
COPY config.example.yaml /etc/yatagarasu/config.yaml.example

# Switch to non-root user
USER yatagarasu

# Expose ports
EXPOSE 8080 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Set working directory
WORKDIR /home/yatagarasu

# Default command
CMD ["yatagarasu", "--config", "/etc/yatagarasu/config.yaml"]

# Metadata
LABEL maintainer="Yatagarasu Team"
LABEL description="High-performance S3 proxy with JWT authentication"
LABEL version="0.2.0-dev"
