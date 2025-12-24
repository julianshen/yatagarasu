# Yatagarasu

> _"The three-legged crow that guides the way to secure S3 access"_

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/julianshen/yatagarasu/actions/workflows/ci.yml/badge.svg)](https://github.com/julianshen/yatagarasu/actions)
[![Docker](https://img.shields.io/badge/docker-ghcr.io-blue.svg)](https://ghcr.io/julianshen/yatagarasu)
[![Version](https://img.shields.io/badge/version-1.5.0-blue.svg)](https://github.com/julianshen/yatagarasu/releases)

[![logo](https://raw.githubusercontent.com/julianshen/yatagarasu/master/logo.png)](https://github.com/julianshen/yatagarasu)

A high-performance **read-only** S3 proxy built with Cloudflare's Pingora framework, providing intelligent routing, multi-bucket support, and flexible authentication for secure content delivery.

## Features

- **High Performance** - Built on Pingora, 70% lower CPU vs traditional proxies
- **Multi-Bucket Routing** - Map different S3 buckets to URL paths with isolated credentials
- **Flexible Authentication** - JWT (HS256/RS256/ES256), JWKS endpoints, OPA/OpenFGA authorization
- **Multi-Tier Caching** - Memory (Moka TinyLFU), Disk, and Redis/Valkey with 80%+ hit rates
- **Image Optimization** - On-the-fly resize, crop, format conversion (WebP/AVIF), effects (blur, sharpen, brightness, contrast, saturation)
- **Server-Side Watermarking** - Text/image watermarks with template variables and 11 positioning modes
- **Production Ready** - Circuit breaker, graceful shutdown, hot reload, distributed tracing
- **Observable** - Prometheus metrics, OpenTelemetry tracing, structured audit logging

## Quick Start

### Docker (Recommended)

```bash
# Pull and run
docker pull ghcr.io/julianshen/yatagarasu:latest
docker run -p 8080:8080 -v ./config.yaml:/etc/yatagarasu/config.yaml \
  ghcr.io/julianshen/yatagarasu:latest

# Or use docker-compose with MinIO for local development
docker compose up -d
curl http://localhost:8080/public/hello.txt
```

### From Source

```bash
git clone https://github.com/julianshen/yatagarasu.git
cd yatagarasu
cargo build --release
./target/release/yatagarasu --config config.example.yaml
```

## Configuration

```yaml
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "public-assets"
    path_prefix: "/assets"
    s3:
      bucket: "my-public-bucket"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY}"
      secret_key: "${AWS_SECRET_KEY}"
    auth:
      enabled: false # Public access

  - name: "private-data"
    path_prefix: "/private"
    s3:
      bucket: "my-private-bucket"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY}"
      secret_key: "${AWS_SECRET_KEY}"
    auth:
      enabled: true
      jwt:
        secret: "${JWT_SECRET}"
        algorithm: "HS256"
        token_sources:
          - type: "bearer"
          - type: "query"
            name: "token"

cache:
  memory:
    max_capacity: 1073741824 # 1GB
    ttl_seconds: 3600

metrics:
  enabled: true
  port: 9090
```

See [config.example.yaml](config.example.yaml) for full configuration reference.

## API

### Supported Methods

| Method    | Description              |
| --------- | ------------------------ |
| `GET`     | Retrieve objects from S3 |
| `HEAD`    | Get object metadata      |
| `OPTIONS` | CORS pre-flight          |

### Endpoints

| Endpoint               | Description                         |
| ---------------------- | ----------------------------------- |
| `/{path_prefix}/*`     | Proxy to configured S3 bucket       |
| `/health`              | Liveness check                      |
| `/ready`               | Readiness check with backend health |
| `/metrics` (port 9090) | Prometheus metrics                  |

### Example Requests

```bash
# Public bucket access
curl http://localhost:8080/assets/image.png

# Authenticated access
curl -H "Authorization: Bearer <jwt>" http://localhost:8080/private/data.json

# Or via query param
curl "http://localhost:8080/private/data.json?token=<jwt>"

# Image optimization (resize, format conversion)
curl "http://localhost:8080/assets/photo.jpg?w=400&h=300&fmt=webp&q=80"

# Image effects (blur, brightness, contrast)
curl "http://localhost:8080/assets/photo.jpg?w=800&blur=3&brightness=10"

# Health check
curl http://localhost:8080/health

# Metrics
curl http://localhost:9090/metrics
```

## Project Structure

```
yatagarasu/
├── src/                    # Rust source code
│   ├── auth/               # JWT, JWKS authentication
│   ├── cache/              # Memory, disk, Redis cache layers
│   ├── config/             # Configuration loading
│   ├── proxy/              # Pingora proxy implementation
│   ├── router/             # Path-to-bucket routing
│   └── s3/                 # S3 client, AWS SigV4
├── tests/                  # Test suite
│   ├── unit/               # Unit tests
│   └── integration/        # Integration tests
├── config/                 # Configuration files
│   └── loadtest/           # Load testing configs
├── docker/                 # Docker compose variants
├── docs/                   # Documentation
├── k6/                     # Load test scripts
└── benches/                # Performance benchmarks
```

## Documentation

| Guide                                             | Description                        |
| ------------------------------------------------- | ---------------------------------- |
| [Getting Started](docs/GETTING_STARTED.md)        | Step-by-step setup guide           |
| [Image Optimization](docs/IMAGE_OPTIMIZATION.md)  | Resize, crop, format conversion    |
| [Watermarking](docs/WATERMARKING.md)              | Text/image watermarks with templates |
| [JWT Authentication](docs/JWT_AUTHENTICATION.md)  | JWT configuration and JWKS         |
| [OPA Policies](docs/OPA_POLICIES.md)              | Policy-based authorization         |
| [OpenFGA](docs/OPENFGA.md)                        | Fine-grained authorization         |
| [Caching](docs/CACHE_MANAGEMENT.md)               | Cache configuration and management |
| [HA & Replication](docs/HA_BUCKET_REPLICATION.md) | Multi-replica failover             |
| [Deployment](docs/DEPLOYMENT.md)                  | Production deployment guide        |
| [Operations](docs/OPERATIONS.md)                  | Monitoring and troubleshooting     |
| [Performance](docs/PERFORMANCE.md)                | Benchmarks and tuning              |
| [Docker](docs/DOCKER.md)                          | Container deployment               |
| [Index](docs/INDEX.md)                            | Full documentation index           |

## Development

```bash
# Run tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt

# Benchmarks
cargo bench
```

See [CLAUDE.md](CLAUDE.md) for TDD methodology and development workflow.

## Operations

### Hot Reload

```bash
# Reload configuration without downtime
kill -HUP $(pgrep yatagarasu)
```

### Graceful Shutdown

```bash
# Complete in-flight requests before stopping
kill -TERM $(pgrep yatagarasu)
```

### Monitoring

- **Metrics**: `http://localhost:9090/metrics` (Prometheus)
- **Health**: `http://localhost:8080/health`
- **Readiness**: `http://localhost:8080/ready`
- **Tracing**: OpenTelemetry export to Jaeger/Zipkin

## Performance

Validated with K6 load testing:

| Metric                | Result   |
| --------------------- | -------- |
| Throughput            | 893+ RPS |
| P95 Latency           | 807µs    |
| Cache Hit Rate        | 80%+     |
| Memory per Connection | ~64KB    |

See [docs/BENCHMARK_RESULTS_V1.2.md](docs/BENCHMARK_RESULTS_V1.2.md) for details.

## License

MIT

## Links

- **Docker Image**: [ghcr.io/julianshen/yatagarasu](https://ghcr.io/julianshen/yatagarasu)
- **Original MVP**: [s3-envoy-proxy](https://github.com/julianshen/s3-envoy-proxy)
- **Pingora Framework**: [cloudflare/pingora](https://github.com/cloudflare/pingora)
