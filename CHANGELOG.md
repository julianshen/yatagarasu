# Changelog

All notable changes to Yatagarasu S3 Proxy will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-11-15

### Added - Production-Ready v1.0 Release

**Core Features**:
- High-performance S3 proxy built on Cloudflare Pingora framework
- Multi-bucket routing with path-based bucket selection
- Per-bucket credential isolation for enhanced security
- JWT-based authentication (optional per-bucket)
- HTTP Range request support for video streaming and parallel downloads
- Zero-copy streaming architecture for large files (constant memory usage)
- Smart caching for small files (<10MB configurable)
- Configuration hot reload via SIGHUP signal and /admin/reload API endpoint

**Authentication & Security**:
- Flexible JWT validation from multiple sources (Authorization header, query params, custom headers)
- Custom claims verification with operators (equals, contains, in, gt, lt)
- Mixed public/private bucket support in single instance
- Per-bucket JWT configuration
- SQL injection prevention in request paths
- Path traversal attack protection
- Read-only enforcement (PUT/POST/DELETE/PATCH blocked)
- Rate limiting (global, per-IP, per-bucket)
- Circuit breaker pattern for backend protection

**Health & Observability**:
- `/health` endpoint for liveness probes
- `/ready` endpoint for readiness probes with S3 backend health checks
- `/metrics` endpoint with comprehensive Prometheus metrics
- Structured logging with request correlation (UUIDs)
- Graceful shutdown with SIGTERM/SIGINT/SIGQUIT handling
- Startup validation for configuration and dependencies

**Performance**:
- Throughput: 726 req/s baseline (test-limited, capable of 1,000+ req/s)
- P95 Latency: 6.7ms (small files), 15.95ms (100 concurrent users)
- Streaming TTFB: 24.45ms P95 (4x better than 100ms target)
- Stability: 1-hour sustained load, zero crashes, 115GB transferred
- Error Rate: 0.00% across all load tests
- Memory: Stable usage, ~60-70MB under sustained load

**Deployment & Operations**:
- Docker support with official Dockerfile
- Docker Compose setup for local development
- Kubernetes-ready (liveness/readiness probes)
- Configuration via YAML with environment variable substitution
- Graceful configuration reload without downtime
- Comprehensive documentation

**Documentation**:
- Complete technical specification (spec.md)
- TDD implementation plan with 200+ tests (plan.md)
- Streaming architecture guide
- Range request documentation
- Cache management guide
- Configuration hot reload guide
- Performance testing report
- Docker deployment guide

**Testing**:
- 200+ unit and integration tests across 25 phases
- Comprehensive K6 load testing suite
- Performance benchmarks documented
- Security testing (SQL injection, path traversal, rate limiting)
- Circuit breaker verification

### Performance Benchmarks

- **Baseline Throughput**: 43,591 requests in 60s (726 req/s), P95 latency 6.7ms
- **Concurrent Connections**: 94,656 requests with 100 VUs, P95 latency 15.95ms
- **Streaming TTFB**: P95 24.45ms (average 14.64ms) for 10MB files
- **Long-Term Stability**: 1 hour sustained load, 50 VUs, ~110,000 requests, 115GB transferred, 0 crashes

### Technical Details

**Architecture**:
- Built on Pingora 0.6 (Cloudflare's production-grade proxy framework)
- Async Rust with Tokio runtime
- AWS SDK for S3 with Signature v4 authentication
- Zero-copy streaming for files >10MB
- LRU cache for files <10MB

**Supported Features**:
- S3 Operations: GET, HEAD (read-only)
- HTTP Methods: GET, HEAD, OPTIONS
- Authentication: JWT (HS256)
- Path Routing: Longest-prefix matching
- Streaming: HTTP/1.1 chunked transfer encoding
- Range Requests: Full HTTP Range header support
- Caching: In-memory with LRU eviction

**Dependencies**:
- Rust 1.70+
- Pingora 0.6
- AWS SDK for Rust
- JSON Web Token (jwt-simple)
- Serde for YAML/JSON

### Breaking Changes
None - this is the initial v1.0 release.

### Upgrade Path
This is the first stable release. Future breaking changes will be avoided whenever possible and clearly documented.

### Known Limitations
- Read-only proxy (write operations intentionally blocked)
- HS256 JWT algorithm only (ES256/RS256 planned for v1.1)
- In-memory caching only (disk/Redis planned for v1.1)
- Single-node deployment (no distributed caching in v1.0)

### Credits
Built with Test-Driven Development (TDD) following Kent Beck's methodology and "Tidy First" principles.

---

## Versioning

This project follows [Semantic Versioning](https://semver.org/):
- **MAJOR** version for incompatible API changes
- **MINOR** version for backwards-compatible functionality additions
- **PATCH** version for backwards-compatible bug fixes

---

**v1.0.0 marks the production-ready release of Yatagarasu S3 Proxy.**

All 25 implementation phases completed with >200 tests passing. Comprehensive performance testing demonstrates production-ready stability with zero crashes during 1-hour sustained load and exceptional latency metrics (6.7ms - 24.45ms P95).
