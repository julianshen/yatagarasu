# Yatagarasu - Product Specification

## Overview

**Product Name:** Yatagarasu (八咫烏)
**Version:** 1.2.0
**Last Updated:** December 2025
**Repository:** https://github.com/julianshen/yatagarasu

Yatagarasu is a high-performance **read-only** S3 proxy built with Cloudflare's Pingora framework and Rust. It provides intelligent routing, multi-bucket support, flexible authentication, and multi-tier caching.

_Yatagarasu (八咫烏) is the three-legged crow in Japanese mythology that serves as a divine guide. Like its namesake, this proxy guides and securely routes requests to S3 buckets._

## Features Summary

### Core Features (v1.0.0)
- Multi-bucket path routing with credential isolation
- JWT authentication (HS256) with multiple token sources
- AWS Signature v4 S3 request signing
- Zero-copy streaming (constant ~64KB memory per connection)
- HTTP Range request support
- Health endpoints (`/health`, `/ready`)
- Prometheus metrics
- Rate limiting and circuit breaker
- HA bucket replication with failover
- Configuration hot reload

### Enhanced Features (v1.1.0)
- Multi-tier caching: Memory (Moka TinyLFU), Disk, Redis/Valkey
- Advanced JWT: RS256, ES256, JWKS endpoints
- OPA policy-based authorization
- Comprehensive audit logging (file, syslog, S3 export)
- OpenTelemetry distributed tracing
- IP allowlist/blocklist (CIDR support)
- Per-user rate limiting

### Production Features (v1.2.0)
- SIGHUP hot reload with ArcSwap
- OpenFGA fine-grained authorization
- Multi-architecture Docker images (GHCR)

## Architecture

```
Client Request
    ↓
Pingora HTTP Server
    ↓
Request Router (path matching)
    ↓
JWT Authenticator (optional)
    ↓
Authorization (OPA/OpenFGA, optional)
    ↓
Cache Check (memory → disk → Redis)
    ↓
S3 Request Builder (AWS SigV4)
    ↓
S3 Backend
    ↓
Response Streamer
    ↓
Client Response
```

## Configuration

```yaml
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "example"
    path_prefix: "/example"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY}"
      secret_key: "${AWS_SECRET_KEY}"
    auth:
      enabled: true
      jwt:
        secret: "${JWT_SECRET}"
        algorithm: "HS256"  # or RS256, ES256
        token_sources:
          - type: "bearer"
          - type: "query"
            name: "token"
    # Optional: OPA authorization
    authorization:
      type: opa
      url: "http://localhost:8181"
      policy_path: "yatagarasu/authz/allow"

cache:
  memory:
    max_capacity: 1073741824  # 1GB
    ttl_seconds: 3600
  # Optional: disk and redis layers

metrics:
  enabled: true
  port: 9090

logging:
  level: "info"
  format: "json"
```

## API Reference

### HTTP Methods

| Method | Supported | Description |
|--------|-----------|-------------|
| GET | ✅ | Retrieve S3 objects |
| HEAD | ✅ | Get object metadata |
| OPTIONS | ✅ | CORS pre-flight |
| PUT/POST/DELETE | ❌ | Blocked (read-only) |

### Endpoints

| Endpoint | Description |
|----------|-------------|
| `/{prefix}/*` | Proxy to configured S3 bucket |
| `/health` | Liveness probe |
| `/ready` | Readiness probe with backend health |
| `:9090/metrics` | Prometheus metrics |

### Response Headers

- `X-Request-ID` - Correlation ID for tracing
- `X-Cache` - HIT/MISS cache status
- `Accept-Ranges: bytes` - Range request support

## Authentication

### JWT Token Sources (checked in order)
1. `Authorization: Bearer <token>` header
2. `?token=<token>` query parameter
3. Custom header (configurable)

### Supported Algorithms
- HS256, HS384, HS512 (symmetric)
- RS256, RS384, RS512 (RSA)
- ES256, ES384 (ECDSA)

### Claims Verification
```yaml
claims_verification:
  - claim: "role"
    operator: "equals"  # equals, contains, in, gt, lt
    value: "admin"
```

## Authorization

### OPA (Open Policy Agent)
```yaml
authorization:
  type: opa
  url: "http://localhost:8181"
  policy_path: "yatagarasu/authz/allow"
  timeout_ms: 100
  cache_ttl_seconds: 60
```

### OpenFGA
```yaml
authorization:
  type: openfga
  url: "http://localhost:8080"
  store_id: "${OPENFGA_STORE_ID}"
```

## Caching

### Cache Layers (checked in order)
1. **Memory** - Moka with TinyLFU eviction
2. **Disk** - Persistent across restarts
3. **Redis/Valkey** - Distributed caching

### Cache Behavior
- Small files (<10MB): Cached
- Large files (>10MB): Always streamed
- Range requests: Never cached
- Cache writes: Async (don't block response)

## Performance

| Metric | Target | Achieved |
|--------|--------|----------|
| Throughput | 1000+ RPS | 893+ RPS |
| P95 Latency (cached) | <10ms | 807µs |
| P95 Latency (S3) | <500ms | <100ms |
| Memory per connection | ~64KB | ~64KB |
| Cache hit rate | >80% | 80%+ |

## Operations

### Hot Reload
```bash
kill -HUP $(pgrep yatagarasu)
```

### Graceful Shutdown
```bash
kill -TERM $(pgrep yatagarasu)
```

### Health Checks
```bash
curl http://localhost:8080/health   # Liveness
curl http://localhost:8080/ready    # Readiness
```

## Error Handling

| Status | Meaning |
|--------|---------|
| 400 | Invalid request (bad path, malformed token) |
| 401 | Missing or invalid JWT |
| 403 | Valid JWT but authorization failed |
| 404 | Object not found in S3 |
| 405 | Method not allowed (PUT/POST/DELETE) |
| 429 | Rate limited |
| 500 | Internal error |
| 502 | S3 backend error |
| 503 | Circuit breaker open |
| 504 | S3 timeout |

## Security

- Per-bucket credential isolation
- JWT validation (not issuance)
- Path traversal protection
- SQL injection prevention
- IP allowlist/blocklist
- Rate limiting (global, per-IP, per-user)
- Sensitive data redaction in logs
- Read-only enforcement

## Technology Stack

- **Language:** Rust 1.70+
- **Framework:** Cloudflare Pingora
- **Runtime:** Tokio
- **S3:** AWS SDK for Rust
- **JWT:** jsonwebtoken crate
- **Cache:** Moka, Redis
- **Metrics:** Prometheus
- **Tracing:** OpenTelemetry

## Deployment

### Docker
```bash
docker pull ghcr.io/julianshen/yatagarasu:1.2.0
docker run -p 8080:8080 -v ./config.yaml:/etc/yatagarasu/config.yaml \
  ghcr.io/julianshen/yatagarasu:1.2.0
```

### Binary
```bash
cargo build --release
./target/release/yatagarasu --config config.yaml
```

## Out of Scope

- S3 write operations (PUT/POST/DELETE)
- JWT token issuance
- Image/video transformation
- CDN/edge caching
- User management
- OAuth/OIDC flows

## References

- [README.md](README.md) - Quick start guide
- [docs/](docs/) - Detailed documentation
- [CLAUDE.md](CLAUDE.md) - Development methodology
- [Pingora](https://github.com/cloudflare/pingora) - Proxy framework
- [AWS S3 API](https://docs.aws.amazon.com/AmazonS3/latest/API/) - S3 reference

---

**Document History**

| Version | Date | Changes |
|---------|------|---------|
| 1.2.0 | Dec 2025 | Hot reload, OpenFGA, GHCR |
| 1.1.0 | Nov 2025 | Caching, RS256/ES256, OPA, audit, tracing |
| 1.0.0 | Nov 2025 | Initial production release |
