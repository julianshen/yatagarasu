# Changelog

All notable changes to Yatagarasu S3 Proxy will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.4.0] - 2025-12-16

### Added

**Zero-Copy File Serving (Linux sendfile)**:
- **sendfile() syscall integration**: Direct kernel-to-kernel data transfer for cached files
- **2.6x throughput improvement**: For files ≥1MB compared to traditional read+write
- **Configurable threshold**: `sendfile.threshold_bytes` (default: 64KB) to control when sendfile is used
- **Platform support**: Full implementation on Linux, graceful fallback on macOS/Windows

**Cache Enhancements**:
- **DiskCache sendfile support**: New `get_sendfile()` method for zero-copy file path retrieval
- **TieredCache integration**: Sendfile support propagated through cache hierarchy
- **SendfileConfig**: New configuration structure for sendfile tuning

**Metrics**:
- **cache_sendfile_count**: Counter for sendfile-eligible cache hits
- **cache_sendfile_bytes**: Counter for bytes served via sendfile

### Changed
- `DiskCacheConfig`: Added `sendfile` field for zero-copy configuration
- `Cache` trait: Added `get_sendfile()` method with default no-op implementation
- Improved cross-platform compatibility with proper `#[cfg]` guards

---

## [1.3.0] - 2025-12-10

### Added

**Cache Warming**:
- **Admin API**: New endpoints (`/admin/cache/prewarm`) for triggering cache warming jobs.
- **Background Worker**: Async worker that lists S3 objects and populates cache layers.
- **Configuration**: New `warming` section in `CacheConfig` for concurrency and rate limiting.
- **Metrics**: Comprehensive metrics (`prewarm_tasks_total`, `prewarm_bytes_total`, etc.) for monitoring.

**Kubernetes Integration**:
- **Helm Chart**: Production-ready Helm chart in `charts/yatagarasu`.
- **Kustomize Overlays**: Environment-specific overlays (`dev`, `prod`, `ha-redis`, `full-stack`).
- **Examples**: Kubernetes manifests covering various deployment scenarios.

**Documentation**:
- **Documentation Website**: Comprehensive guide built with `mdBook`.
- **Tutorials**: Guides for Authentication, Caching, and Basic Setup.
- **API Reference**: Detailed documentation of the Admin API.

### Changed
- `CacheConfig`: Added `warming` field (backward compatible).
- `S3Config`: Improved struct definition for better maintainability.
- `S3 Client`: Added `ListObjectsV2` support with filtering and pagination.

---

## [1.2.0] - 2025-12-08

### Added

**Hot Reload**:
- SIGHUP signal handler for configuration hot reload without downtime
- ArcSwap-based atomic configuration swapping (lock-free reads)
- Zero dropped requests during reload

**OpenFGA Authorization**:
- Fine-grained authorization with OpenFGA integration
- Relationship-based access control (ReBAC)
- Configurable OpenFGA server connection

**Production Hardening**:
- Enhanced CI pipeline stability
- Multi-architecture Docker images (amd64, arm64)
- Published to GitHub Container Registry (ghcr.io)

**Project Organization**:
- Reorganized project structure (config/, docker/, docs/archive/)
- Consolidated load test configurations
- Cleaned up documentation

### Changed
- Docker image now available at `ghcr.io/julianshen/yatagarasu:1.2.0`
- Improved test reliability in CI environments

---

## [1.1.0] - 2025-11-30

### Added

**Multi-Tier Caching**:
- In-memory LRU cache with Moka (TinyLFU eviction)
- Disk cache layer for persistence across restarts
- Redis/Valkey distributed cache support
- Tiered cache hierarchy (memory → disk → Redis)
- 80%+ cache hit rates for static workloads
- Cache purge and statistics APIs

**Advanced JWT Authentication**:
- RS256 (RSA) algorithm support
- ES256 (ECDSA) algorithm support
- JWKS endpoint integration for key rotation
- Multiple key support

**OPA Authorization**:
- Policy-based access control with Open Policy Agent
- Rego policy language support
- Decision caching for performance
- Configurable fail modes (open/closed)

**Audit Logging**:
- Comprehensive request audit logging
- Structured JSON format with correlation IDs
- Multiple outputs: file, syslog, S3 export
- Sensitive data redaction
- Configurable retention and rotation

**Enhanced Observability**:
- OpenTelemetry distributed tracing
- OTLP, Jaeger, and Zipkin exporters
- Slow query logging with configurable thresholds
- Request/response logging with filtering

**Advanced Security**:
- IP allowlist/blocklist per bucket (CIDR support)
- Per-user rate limiting from JWT claims
- Enhanced DDoS protection

### Performance
- Throughput: 893+ RPS validated
- P95 Latency: 807µs (cached content)
- Cache hit rate: 80%+ for static assets
- Memory efficient: ~64KB per streaming connection

---

## [1.0.0] - 2025-11-15

### Added

**Core Features**:
- High-performance S3 proxy built on Cloudflare Pingora framework
- Multi-bucket routing with path-based bucket selection
- Per-bucket credential isolation for enhanced security
- JWT-based authentication (HS256, optional per-bucket)
- HTTP Range request support for video streaming
- Zero-copy streaming architecture (constant ~64KB memory per connection)

**Authentication & Security**:
- Flexible JWT validation from multiple sources (header, query, custom)
- Custom claims verification with operators (equals, contains, in, gt, lt)
- Mixed public/private bucket support
- SQL injection prevention
- Path traversal attack protection
- Read-only enforcement (PUT/POST/DELETE/PATCH blocked)
- Rate limiting (global, per-IP, per-bucket)
- Circuit breaker pattern

**Health & Observability**:
- `/health` endpoint for liveness probes
- `/ready` endpoint for readiness probes with backend health
- `/metrics` endpoint with Prometheus metrics
- Structured JSON logging with request correlation (UUIDs)
- Graceful shutdown (SIGTERM/SIGINT/SIGQUIT)

**High Availability**:
- HA bucket replication with automatic failover
- Priority-based replica selection
- Per-replica health monitoring
- Circuit breaker integration

**Deployment**:
- Docker support with multi-stage Dockerfile
- Docker Compose for local development
- Kubernetes-ready (liveness/readiness probes)
- YAML configuration with environment variable substitution

### Performance
- Throughput: 726+ req/s baseline
- P95 Latency: 6.7ms (small files)
- Streaming TTFB: 24.45ms P95
- Stability: 1-hour sustained load, zero crashes
- Error Rate: 0.00%

---

## Versioning

This project follows [Semantic Versioning](https://semver.org/):
- **MAJOR** version for incompatible API changes
- **MINOR** version for backwards-compatible functionality additions
- **PATCH** version for backwards-compatible bug fixes
