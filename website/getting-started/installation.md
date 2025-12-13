---
title: Installation
layout: default
parent: Getting Started
nav_order: 1
---

# Installation

Multiple ways to install Yatagarasu depending on your needs.
{: .fs-6 .fw-300 }

---

## Docker (Recommended)

The easiest way to run Yatagarasu in production.

### Pull from GitHub Container Registry

```bash
# Latest stable release
docker pull ghcr.io/julianshen/yatagarasu:latest

# Latest development
docker pull ghcr.io/julianshen/yatagarasu:latest
```

### Available Tags

| Tag | Description |
|:----|:------------|
| `1.2.0` | Latest stable release |
| `1.1.0` | Previous stable release |
| `latest` | Latest build from main branch |
| `sha-<commit>` | Specific commit builds |

### Run with Docker

```bash
# Create a configuration file
cat > config.yaml << 'EOF'
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "my-bucket"
    path_prefix: "/files"
    s3:
      bucket: "my-s3-bucket"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY_ID}"
      secret_key: "${AWS_SECRET_ACCESS_KEY}"
    auth:
      enabled: false

metrics:
  enabled: true
  port: 9090
EOF

# Run the container
docker run -d \
  --name yatagarasu \
  -p 8080:8080 \
  -p 9090:9090 \
  -v $(pwd)/config.yaml:/etc/yatagarasu/config.yaml \
  -e AWS_ACCESS_KEY_ID=your-access-key \
  -e AWS_SECRET_ACCESS_KEY=your-secret-key \
  ghcr.io/julianshen/yatagarasu:latest

# Verify it's running
curl http://localhost:8080/health
```

---

## Build from Source

Build Yatagarasu from source for development or customization.

### Prerequisites

- **Rust 1.70+** - Install via [rustup](https://rustup.rs/)
- **Git** - For cloning the repository

### Build Steps

```bash
# Clone the repository
git clone https://github.com/julianshen/yatagarasu.git
cd yatagarasu

# Build release binary (optimized)
cargo build --release

# The binary is at ./target/release/yatagarasu
ls -la target/release/yatagarasu
```

### Run from Source

```bash
# Run with a configuration file
./target/release/yatagarasu --config config.yaml

# Or run directly with cargo
cargo run --release -- --config config.yaml
```

### Development Build

For faster compilation during development:

```bash
# Debug build (faster compile, slower runtime)
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- --config config.yaml
```

---

## Pre-built Binaries

Download pre-built binaries from GitHub Releases.

### Download

```bash
# Linux x86_64
curl -LO https://github.com/julianshen/yatagarasu/releases/download/v1.2.0/yatagarasu-linux-x86_64.tar.gz
tar xzf yatagarasu-linux-x86_64.tar.gz

# Linux ARM64
curl -LO https://github.com/julianshen/yatagarasu/releases/download/v1.2.0/yatagarasu-linux-aarch64.tar.gz
tar xzf yatagarasu-linux-aarch64.tar.gz

# macOS x86_64
curl -LO https://github.com/julianshen/yatagarasu/releases/download/v1.2.0/yatagarasu-darwin-x86_64.tar.gz
tar xzf yatagarasu-darwin-x86_64.tar.gz

# macOS ARM64 (Apple Silicon)
curl -LO https://github.com/julianshen/yatagarasu/releases/download/v1.2.0/yatagarasu-darwin-aarch64.tar.gz
tar xzf yatagarasu-darwin-aarch64.tar.gz
```

### Install System-wide

```bash
# Move to system path
sudo mv yatagarasu /usr/local/bin/

# Verify installation
yatagarasu --version
```

---

## Kubernetes (Helm)

Install Yatagarasu on Kubernetes using Helm.

### Add Helm Repository

```bash
# Add the repository
helm repo add yatagarasu https://julianshen.github.io/yatagarasu/charts
helm repo update
```

### Install

```bash
# Install with default values
helm install yatagarasu yatagarasu/yatagarasu

# Install with custom values
helm install yatagarasu yatagarasu/yatagarasu \
  --set replicaCount=3 \
  --set config.buckets[0].name=my-bucket \
  --set config.buckets[0].pathPrefix=/files

# Install from local chart
helm install yatagarasu ./charts/yatagarasu -f values.yaml
```

See [Kubernetes Deployment](/yatagarasu/deployment/kubernetes/) for detailed configuration.

---

## Verify Installation

Regardless of installation method, verify Yatagarasu is working:

```bash
# Health check
curl http://localhost:8080/health
# Expected: {"status":"ok"}

# Readiness check (includes backend health)
curl http://localhost:8080/ready
# Expected: {"status":"ok","backends":[...]}

# Prometheus metrics
curl http://localhost:9090/metrics
# Expected: Prometheus metrics output
```

---

## System Requirements

### Minimum

- **CPU**: 1 core
- **Memory**: 256MB RAM
- **Disk**: 100MB for binary + cache storage

### Recommended for Production

- **CPU**: 2+ cores
- **Memory**: 1GB+ RAM (depends on cache size)
- **Disk**: SSD for disk cache
- **Network**: Low-latency connection to S3

### Resource Scaling

| Concurrent Connections | Recommended Memory |
|:-----------------------|:-------------------|
| 100 | 256MB |
| 1,000 | 512MB |
| 10,000 | 1GB |
| 100,000 | 2GB + increased ulimits |

Memory usage is primarily determined by:
- Cache size configuration
- Number of concurrent connections (~64KB per connection for streaming)

---

## Configuration File Location

Yatagarasu looks for configuration in these locations (in order):

1. Path specified via `--config` flag
2. `/etc/yatagarasu/config.yaml`
3. `./config.yaml` (current directory)

```bash
# Explicit path
yatagarasu --config /path/to/config.yaml

# Default locations
yatagarasu  # Will look for /etc/yatagarasu/config.yaml or ./config.yaml
```

---

## Next Steps

- [Docker Quickstart](/yatagarasu/getting-started/docker-quickstart/) - Complete Docker setup guide
- [Docker Compose](/yatagarasu/getting-started/docker-compose/) - Development environment with MinIO
- [Configuration Reference](/yatagarasu/configuration/) - All configuration options
