---
title: Home
layout: home
nav_order: 1
description: "Yatagarasu - High-performance S3 proxy built with Rust and Cloudflare Pingora"
permalink: /
---

# Yatagarasu

**The three-legged crow that guides the way to secure S3 access**
{: .fs-6 .fw-300 }

Yatagarasu is a high-performance, read-only S3 proxy built with Rust and [Cloudflare Pingora](https://github.com/cloudflare/pingora). It provides intelligent routing, multi-bucket support, flexible authentication, and enterprise-grade caching for secure content delivery.

[Get Started](/yatagarasu/getting-started/){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[View on GitHub](https://github.com/julianshen/yatagarasu){: .btn .fs-5 .mb-4 .mb-md-0 }

---

## Why Yatagarasu?

Traditional S3 access patterns expose your backend infrastructure and credentials directly to clients. Yatagarasu acts as a secure gateway, providing:

- **Credential Isolation** - S3 credentials never reach your clients
- **Flexible Authentication** - JWT, JWKS, OPA, or OpenFGA authorization per bucket
- **Performance at Scale** - Zero-copy streaming, multi-tier caching, 893+ RPS throughput
- **Production Ready** - Circuit breakers, graceful shutdown, hot reload, observability

## Key Features

### Multi-Bucket Routing

Route different URL paths to different S3 buckets with isolated credentials per bucket.

```yaml
buckets:
  - name: "public-assets"
    path_prefix: "/assets"
    s3:
      bucket: "public-bucket"
    auth:
      enabled: false

  - name: "private-data"
    path_prefix: "/private"
    s3:
      bucket: "private-bucket"
    auth:
      enabled: true
      jwt:
        secret: "${JWT_SECRET}"
```

### Flexible Authentication

Support for multiple authentication methods per bucket:

| Method | Description |
|:-------|:------------|
| **JWT (HS256/RS256/ES256)** | Token-based authentication with custom claims verification |
| **JWKS Endpoints** | Dynamic key rotation via JSON Web Key Sets |
| **OPA Policies** | Policy-based authorization with Open Policy Agent |
| **OpenFGA** | Fine-grained ReBAC (Relationship-Based Access Control) |

### Multi-Tier Caching

Achieve 80%+ cache hit rates with intelligent tiered caching:

```
Client Request
    |
    v
+-------------------+
|   Memory Cache    |  <-- L1: ~320ns access, TinyLFU eviction
|   (Moka)          |
+-------------------+
    |
    v
+-------------------+
|   Redis/Valkey    |  <-- L2: Distributed, shared across instances
|   Cache           |
+-------------------+
    |
    v
+-------------------+
|   Disk Cache      |  <-- L3: Large capacity, persistent
+-------------------+
    |
    v
+-------------------+
|   S3 Backend      |  <-- Origin
+-------------------+
```

### Zero-Copy Streaming

Stream large files (GB+) with constant ~64KB memory per connection:

```
+--------+     +-----------+     +--------+
| Client | <-- | Yatagarasu| <-- |   S3   |
+--------+     +-----------+     +--------+
                    |
                    | ~64KB buffer
                    | (constant memory)
```

### High Availability

Built-in replica failover with circuit breakers and health checking:

```yaml
s3:
  replicas:
    - name: primary
      bucket: main-bucket
      region: us-west-2
      priority: 1
    - name: backup
      bucket: backup-bucket
      region: us-east-1
      priority: 2
  circuit_breaker:
    failure_threshold: 5
    timeout_seconds: 30
```

## Performance

Validated benchmarks with K6 load testing:

| Metric | Result |
|:-------|:-------|
| **Throughput** | 893+ RPS |
| **P95 Latency (cached)** | 807us |
| **P95 TTFB (S3 stream)** | 24.45ms |
| **Cache Hit Rate** | 80%+ |
| **Memory per Connection** | ~64KB |
| **JWT Validation** | 1.78us |
| **Path Routing** | 95.9ns |

## Quick Example

```bash
# Pull the Docker image
docker pull ghcr.io/julianshen/yatagarasu:latest

# Run with your configuration
docker run -p 8080:8080 \
  -v ./config.yaml:/etc/yatagarasu/config.yaml \
  ghcr.io/julianshen/yatagarasu:latest

# Access your S3 content
curl http://localhost:8080/assets/image.png

# Authenticated access
curl -H "Authorization: Bearer <jwt>" \
  http://localhost:8080/private/data.json
```

## Architecture Overview

```
                                    +------------------+
                                    |   S3 Backend 1   |
                                    +------------------+
                                           ^
                                           |
+--------+     +-------------------+       |      +------------------+
|        |     |                   |  +----+----+ |   S3 Backend 2   |
| Client | --> |    Yatagarasu     |  |         | +------------------+
|        |     |                   |--| Router  |        ^
+--------+     |  +-------------+  |  |         |        |
               |  | JWT/Auth    |  |  +---------+  +-----+-----+
               |  +-------------+  |               | Replica   |
               |  | Cache Layer |  |               | Selection |
               |  +-------------+  |               +-----------+
               |  | Metrics     |  |
               |  +-------------+  |
               +-------------------+
                        |
                        v
               +-------------------+
               |    Prometheus     |
               |    /metrics       |
               +-------------------+
```

## Deployment Options

Yatagarasu supports multiple deployment scenarios:

| Deployment | Best For |
|:-----------|:---------|
| **Docker** | Development, single-instance production |
| **Docker Compose** | Multi-service development, testing |
| **Kubernetes (Helm)** | Production with auto-scaling |
| **Kubernetes (Kustomize)** | GitOps workflows |

## Compatibility

### S3-Compatible Storage

Works with any S3-compatible storage:

- **AWS S3**
- **MinIO**
- **Cloudflare R2**
- **DigitalOcean Spaces**
- **Backblaze B2**
- **Wasabi**
- **LocalStack** (for testing)

### Cache Backends

- **Memory** (built-in Moka cache)
- **Redis** 6.0+
- **Valkey** 7.0+
- **KeyDB**

## Getting Help

- [GitHub Issues](https://github.com/julianshen/yatagarasu/issues) - Bug reports and feature requests
- [Discussions](https://github.com/julianshen/yatagarasu/discussions) - Questions and community support
- [Documentation](/yatagarasu/getting-started/) - Comprehensive guides and references

## License

Yatagarasu is released under the [MIT License](https://github.com/julianshen/yatagarasu/blob/main/LICENSE).
