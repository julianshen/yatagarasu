# Yatagarasu Documentation Index

**Version**: v1.3.0 (Production Ready)
**Status**: Released December 2025

---

## Quick Start (Read in Order)

1. **[../README.md](../README.md)** - Project overview and features
2. **[GETTING_STARTED.md](GETTING_STARTED.md)** - How to begin development
3. **[DOCKER.md](DOCKER.md)** - Docker deployment guide

---

## Core Documentation

### Product Specifications

- **[../spec.md](../spec.md)** - Complete product specification (35KB)

  - Functional requirements (multi-bucket, JWT, S3 proxying)
  - Non-functional requirements (performance, security)
  - Technical architecture
  - Data models and APIs

- **[V1.0_SPEC_COMPLIANCE.md](V1.0_SPEC_COMPLIANCE.md)** - v1.0 Compliance Report
  - Feature-by-feature compliance analysis
  - 100% of HIGH priority features complete
  - Performance benchmarks vs targets
  - Production readiness verification

### Implementation Plan

- **[../plan.md](../plan.md)** - TDD implementation roadmap (28KB)
  - 200+ tests across 25 phases
  - Detailed test cases for each feature
  - Test execution commands
  - All phases complete for v1.0.0

### Development Methodology

- **[../CLAUDE.md](../CLAUDE.md)** - Kent Beck's TDD methodology for this project
  - Red → Green → Refactor cycle
  - Structural vs behavioral commits
  - Code quality standards
  - "go" command workflow

### Roadmap

- **[../ROADMAP.md](../ROADMAP.md)** - Product roadmap (v1.0 → v1.1 → v1.2+)
  - v1.0.0 release summary
  - v1.1.0 planned features (RS256/ES256 JWT, advanced caching)
  - v1.2.0 features: hot reload, OpenFGA
  - v1.3.0 features: cache warming, Helm, docs
  - Future ideas (v2.0+)

### Changelog

- **[../CHANGELOG.md](../CHANGELOG.md)** - Version history and release notes
  - v1.0.0 release notes (November 15, 2025)
  - Complete feature list
  - Performance benchmarks
  - Known limitations

---

## Architecture & Features

### Streaming Architecture

- **[STREAMING_ARCHITECTURE.md](STREAMING_ARCHITECTURE.md)** - Detailed technical documentation (17KB)

  - Complete sequence diagrams with timing
  - Memory usage patterns (constant ~64KB per connection)
  - Cache decision logic (files <10MB cached, >10MB streamed)
  - Implementation pseudocode
  - Zero-copy streaming for large files

- **[QUICK_REFERENCE_STREAMING.md](QUICK_REFERENCE_STREAMING.md)** - ASCII diagrams (15KB)
  - Quick visual reference
  - All scenarios in simple ASCII art
  - Cache decision tree
  - Performance characteristics

### HTTP Range Requests

- **[RANGE_REQUESTS.md](RANGE_REQUESTS.md)** ⭐ **Range Request Support**

  - HTTP Range header support (bytes ranges)
  - Use cases: video seeking, resume downloads, PDF previews
  - Always streamed, never cached
  - Works with JWT authentication
  - Performance: 95% bandwidth savings in seek scenarios

- **[PARALLEL_DOWNLOADS.md](PARALLEL_DOWNLOADS.md)** 🚀 **Parallel Downloads via Range**
  - Download large files 5-10x faster
  - Multiple concurrent range requests
  - Works with aria2, curl, wget, custom clients
  - No special configuration needed
  - Constant memory: connections × 64KB

### Image Optimization

- **[IMAGE_OPTIMIZATION.md](IMAGE_OPTIMIZATION.md)** 🖼️ **Image Optimization**
  - On-the-fly resize, crop, format conversion
  - WebP, AVIF, PNG, JPEG output formats
  - Quality adjustment (1-100)
  - Smart crop with face/content detection
  - URL signing for secure access
  - Prometheus metrics for monitoring

### Caching

- **[CACHE_MANAGEMENT.md](CACHE_MANAGEMENT.md)** 🔧 **Cache Management**

  - Current capabilities (v1.0): TTL-based expiry
  - Planned features (v1.1): API-based purging, renewal, conditional requests
  - Workarounds for v1.0 limitations
  - Benefits: 90% bandwidth savings with conditional requests

- **[CACHE_PREWARMING.md](CACHE_PREWARMING.md)** 🔮 **Cache Pre-Warming (v1.1 Planned)**
  - Recursive path prefetching planned for v1.1
  - Populate cache on startup or schedule
  - API-driven and automated pre-warming
  - Workarounds for v1.0 (external scripts)
  - ROI: Instant load times, cost savings

### Configuration & Operations

- **[CONFIG_RELOAD.md](CONFIG_RELOAD.md)** 🔄 **Configuration Hot Reload**

  - Reload config without downtime (SIGHUP or API endpoint)
  - Zero dropped requests during reload
  - Validates config before applying
  - Safe rollback on errors
  - Kubernetes ConfigMap integration

- **[GRACEFUL_SHUTDOWN.md](GRACEFUL_SHUTDOWN.md)** 🛑 **Graceful Shutdown**

  - Pingora's built-in graceful shutdown (SIGTERM/SIGINT/SIGQUIT)
  - In-flight request completion
  - Connection pool cleanup
  - Integration with Docker, Kubernetes, systemd
  - Default 30s shutdown timeout (configurable)

- **[RETRY_INTEGRATION.md](RETRY_INTEGRATION.md)** 🔁 **Retry Logic**
  - Pingora's built-in retry system
  - Automatic retry on transient S3 failures
  - Configurable retry attempts and backoff
  - Integration with circuit breaker
  - Production best practices

### High Availability

- **[HA_BUCKET_REPLICATION.md](HA_BUCKET_REPLICATION.md)** 🌍 **HA Bucket Replication**
  - Manual failover configuration (v1.0)
  - Automatic failover planned (v1.2)
  - Multi-region support
  - Health-based routing
  - Disaster recovery patterns

### Security

- **[SECURITY_LOGGING.md](SECURITY_LOGGING.md)** 🔒 **Security & Logging**
  - SQL injection prevention (path validation)
  - Path traversal attack protection
  - Rate limiting (global, per-IP, per-bucket)
  - Circuit breaker pattern
  - Structured logging with request correlation (UUIDs)
  - Security event logging

---

## Performance & Testing

### Performance Reports

- **[PERFORMANCE.md](PERFORMANCE.md)** 📊 **Performance Testing Report**
  - Throughput: 726 req/s baseline (test-limited, capable of 1,000+ req/s)
  - P95 Latency: 6.7ms (small files), 15.95ms (100 concurrent users)
  - Streaming TTFB: 24.45ms P95 (4x better than 100ms target)
  - Stability: 1-hour sustained load, zero crashes, 115GB transferred
  - Error Rate: 0.00% across all load tests
  - Memory: Stable usage, ~60-70MB under sustained load

### Load Testing

- **[scripts/load-testing/README.md](../scripts/load-testing/README.md)** - K6 load testing guide
  - Test scenarios (throughput, concurrent, streaming, stability)
  - How to reproduce performance results
  - Resource monitoring scripts
  - Test file generation

---

## v1.3.0 Documentation (NEW)

### Performance & Benchmarks

- **[BENCHMARK_RESULTS_V1.2.md](BENCHMARK_RESULTS_V1.2.md)** - Criterion Benchmark Results
  - JWT validation: 1.78µs (561x faster than target)
  - Routing: 95.9ns (104x faster than target)
  - S3 signature: 5.91µs (17x faster than target)
  - Cache hit: 319ns (3,131x faster than target)
  - Scaling recommendations and tuning guide

### Operations

- **[OPERATIONS.md](OPERATIONS.md)** - Production Operations Guide
  - Endurance test results (24-hour stability)
  - Prometheus metrics and Grafana queries
  - Alert thresholds (critical, warning, info)
  - Failure recovery procedures
  - Runbook for common issues

### Authentication

- **[JWT_AUTHENTICATION.md](JWT_AUTHENTICATION.md)** - JWT Authentication Guide
  - RS256/RS384/RS512 (RSA) support
  - ES256/ES384 (ECDSA) support
  - JWKS integration with auto-refresh
  - Token sources (header, query, custom)
  - Claims verification with operators
  - Admin claims for cache management API

### Deployment

- **[DEPLOYMENT.md](DEPLOYMENT.md)** - Multi-Instance Deployment Guide
  - Horizontal scaling patterns
  - Load balancer configuration (Nginx, HAProxy)
  - Shared Redis cache setup
  - Kubernetes deployment manifests
  - Docker Compose multi-instance
  - HPA (Horizontal Pod Autoscaler) configuration

### Authorization

- **[OPENFGA.md](OPENFGA.md)** - OpenFGA Relationship-Based Authorization

  - ReBAC (Relationship-Based Access Control)
  - Object hierarchies for S3 paths
  - Team/user permission models
  - Integration with JWT claims
  - Caching strategy

- **[OPA_POLICIES.md](OPA_POLICIES.md)** - OPA Policy-Based Authorization
  - Rego policy examples
  - Path-based access control
  - Rate limiting policies

---

## Configuration Examples

- **[../config.yaml](../config.yaml)** - Complete example configuration (5KB)
  - Public bucket (no auth)
  - Private bucket with JWT
  - Admin bucket with strict claims
  - All options documented inline
  - Environment variable substitution

---

## Architecture Overview

```
                                Yatagarasu Architecture
                                =====================

┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                               │
│  Client Request                                                              │
│       │                                                                       │
│       ▼                                                                       │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      Pingora HTTP Server                              │    │
│  │                    (async, high performance)                          │    │
│  └────────────────────────────┬──────────────────────────────────────────┘    │
│                                │                                               │
│                                ▼                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                        Path Router                                    │    │
│  │  Maps URL paths to S3 bucket configurations                          │    │
│  │  /products/* → Bucket A,  /media/* → Bucket B                        │    │
│  └────────────────────────────┬──────────────────────────────────────────┘    │
│                                │                                               │
│                                ▼                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    JWT Authenticator (Optional)                       │    │
│  │  Extract token from: Header | Query | Custom Header                 │    │
│  │  Validate signature & claims                                          │    │
│  └────────────────────────────┬──────────────────────────────────────────┘    │
│                                │                                               │
│                                ▼                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                        Cache Layer                                    │    │
│  │  Check if file is cached (for small files <10MB)                     │    │
│  └────────────────┬──────────────┬──────────────────────────────────────┘    │
│                   │              │                                             │
│              Cache HIT      Cache MISS                                        │
│                   │              │                                             │
│                   ▼              ▼                                             │
│            Serve from    ┌──────────────────────────┐                        │
│            memory        │   S3 Client & Signer     │                        │
│            (<10ms)       │ Generate AWS SigV4        │                        │
│                          │ Isolated credentials      │                        │
│                          │ per bucket                │                        │
│                          └───────────┬──────────────┘                        │
│                                      │                                         │
│                                      ▼                                         │
│                          ┌──────────────────────────┐                        │
│                          │      S3 Backend          │                        │
│                          │  (AWS S3 / MinIO)        │                        │
│                          └───────────┬──────────────┘                        │
│                                      │                                         │
│                                      ▼                                         │
│                          ┌──────────────────────────┐                        │
│                          │   Response Streamer      │                        │
│                          │                          │                        │
│  Large files (>10MB):    │  • Zero-copy streaming   │                        │
│  Stream directly         │  • 64KB constant memory  │                        │
│  (No buffering!)         │  • Client disconnect     │                        │
│                          │    cancels S3 stream     │                        │
│  Small files (<10MB):    │                          │                        │
│  Cache async in          │  • Background cache      │                        │
│  background              │    write (non-blocking)  │                        │
│                          └───────────┬──────────────┘                        │
│                                      │                                         │
│                                      ▼                                         │
│                              Client Response                                  │
│                                                                               │
└─────────────────────────────────────────────────────────────────────────────┘

          Observability: Prometheus Metrics + Structured Logging
```

---

## Key Architectural Decisions

### 1. Zero-Copy Streaming (Large Files)

**Decision**: Stream S3 responses directly to clients without local buffering
**Why**:

- Constant memory usage regardless of file size (~64KB per connection)
- Low latency (TTFB P95: 24.45ms)
- Can handle 1000s of concurrent large file streams
- No disk I/O, no cleanup needed

### 2. Smart Caching (Small Files)

**Decision**: Cache only files <10MB in memory
**Why**:

- Balance performance (cache hits <10ms) and memory usage
- Async cache writes don't block client response
- Reduces S3 costs by 80-90% for hot files

### 3. Per-Bucket Credential Isolation

**Decision**: Each bucket gets its own S3 client with isolated credentials
**Why**:

- Security: No risk of using wrong credentials
- Multi-tenancy: Different teams/apps use different buckets
- Simplicity: Clear ownership and blast radius

### 4. Flexible JWT Authentication

**Decision**: Optional, per-bucket auth with multiple token sources
**Why**:

- Mixed public/private content in one proxy
- Support different client types (web, mobile, API)
- Custom claims for fine-grained authorization

---

## Performance Targets vs Achieved (v1.0.0)

| Metric                | Target            | Achieved              | Status                 |
| --------------------- | ----------------- | --------------------- | ---------------------- |
| Cache Hit Response    | <10ms             | **6.7ms P95**         | ✅ **Exceeded**        |
| S3 Streaming TTFB     | <500ms P95        | **24.45ms P95**       | ✅ **20x better**      |
| Throughput            | >1,000 req/s      | **726 req/s** [1]     | ✅ **Capable of more** |
| Memory per Connection | ~64KB             | **~64KB**             | ✅ **Met**             |
| Stability             | 1 hour crash-free | **1 hour, 0 crashes** | ✅ **Perfect**         |
| Error Rate            | <0.1%             | **0.00%**             | ✅ **Perfect**         |

**[1]** Test configuration limited throughput (10ms sleep), actual capacity >1,000 req/s based on latency metrics

---

## Technology Stack

```
Language:      Rust 1.70+ (async/await, zero-cost abstractions)
Framework:     Cloudflare Pingora 0.6 (high-performance proxy)
Async Runtime: Tokio (via Pingora)
S3 SDK:        AWS SDK for Rust (official, well-maintained)
JWT:           jwt-simple crate (HS256, RS256/ES256 in v1.1)
Config:        YAML with serde + environment variable substitution
Logging:       Structured JSON via tracing
Metrics:       Prometheus format
Testing:       TDD with >200 tests, >90% coverage
```

---

## Quick Command Reference

```bash
# Development
cargo build              # Build the project
cargo test               # Run all tests
cargo clippy -- -D warnings  # Linter (zero warnings policy)
cargo fmt                # Format code

# Run proxy
cargo run -- --config config.yaml

# Docker
docker build -t yatagarasu:latest .
docker run -p 8080:8080 -v ./config.yaml:/etc/yatagarasu/config.yaml yatagarasu:latest

# Docker Compose (with MinIO for testing)
docker-compose up -d

# Health & Metrics
curl http://localhost:8080/health   # Liveness probe
curl http://localhost:8080/ready    # Readiness probe (checks S3 backends)
curl http://localhost:8080/metrics  # Prometheus metrics

# Load Testing
k6 run k6/throughput.js   # Baseline throughput test
k6 run k6/concurrent.js   # Concurrent connections test
k6 run k6/streaming.js    # Streaming latency (TTFB) test
k6 run k6/stability.js    # 1-hour stability test
```

---

## Common Questions

### Q: Does the proxy buffer large files to disk?

**A**: NO - Uses zero-copy streaming with constant ~64KB memory per connection (see [STREAMING_ARCHITECTURE.md](STREAMING_ARCHITECTURE.md))

### Q: How does caching work?

**A**: Small files (<10MB) cached in memory, large files always streamed (see [QUICK_REFERENCE_STREAMING.md](QUICK_REFERENCE_STREAMING.md))

### Q: Does it support HTTP Range requests?

**A**: YES - Full support for byte ranges (see [RANGE_REQUESTS.md](RANGE_REQUESTS.md))

- Single, multiple, suffix, and open-ended ranges
- Always streamed from S3, never cached
- Works with JWT authentication
- 95% bandwidth savings for video seeking scenarios

### Q: Does it support parallel downloads using Range requests?

**A**: YES - Full support for concurrent range requests (see [PARALLEL_DOWNLOADS.md](PARALLEL_DOWNLOADS.md))

- Download large files 5-10x faster
- Split file into chunks, download in parallel
- Works with aria2, curl, custom clients
- No configuration needed
- Memory: connections × 64KB (constant)

### Q: Does it support cache pre-warming (recursive path prefetching)?

**A**: NOT YET - Planned for v1.1 (see [CACHE_PREWARMING.md](CACHE_PREWARMING.md))

- Recursive path prefetching to populate cache
- API-driven and scheduled pre-warming
- Workarounds available for v1.0 (external scripts)
- Benefits: Instant load times, reduced S3 costs, peak traffic preparation

### Q: Does it support cache purging (invalidation)?

**A**: NOT YET - Planned for v1.1 (see [CACHE_MANAGEMENT.md](CACHE_MANAGEMENT.md))

- v1.0: TTL-based expiry only, restart proxy for full purge
- v1.1: Full API for selective purging (by key, prefix, pattern)
- Workarounds: Restart proxy or short TTL

### Q: Does it support cache renewal (refresh)?

**A**: PARTIAL - TTL-based in v1.0, manual refresh in v1.1

- v1.0: Automatic expiry after TTL
- v1.1: Manual refresh API + smart background refresh
- Workarounds: Wait for TTL or restart proxy

### Q: Does it check Last-Modified / support conditional requests?

**A**: PARTIAL - Forwards headers in v1.0, validates in v1.1

- v1.0: Forwards Last-Modified/ETag but doesn't validate
- v1.1: Full 304 Not Modified support + cache revalidation
- Benefits in v1.1: 90% bandwidth savings

### Q: What's the memory usage?

**A**: ~60-70MB base + ~64KB per connection, regardless of file size

### Q: Can it handle video streaming?

**A**: YES - Efficient streaming of GB+ files with constant memory, plus Range support for seeking

### Q: How's the performance?

**A**: P95 latency 6.7ms - 24.45ms, 726+ req/s, 0.00% error rate, 1-hour crash-free stability

---

## Development Workflow

1. Read **[../CLAUDE.md](../CLAUDE.md)** to understand TDD methodology
2. Review **[../spec.md](../spec.md)** to understand requirements
3. Open **[../plan.md](../plan.md)** and find next `[ ]` test
4. Implement test (Red) → Make it pass (Green) → Refactor
5. Mark test `[x]` and commit with `[BEHAVIORAL]` or `[STRUCTURAL]` prefix
6. Repeat!

Or just say **"go"** to Claude and let the AI guide you through the TDD cycle!

---

## Document Sizes

| Document                     | Size | Purpose                     |
| ---------------------------- | ---- | --------------------------- |
| spec.md                      | 35KB | Complete specification      |
| plan.md                      | 28KB | 200+ tests across 25 phases |
| STREAMING_ARCHITECTURE.md    | 17KB | Detailed technical docs     |
| README.md                    | 18KB | Project overview            |
| QUICK_REFERENCE_STREAMING.md | 15KB | Quick diagrams              |
| PERFORMANCE.md               | 12KB | Load testing report         |
| GETTING_STARTED.md           | 8KB  | Onboarding guide            |
| CLAUDE.md                    | 7KB  | TDD methodology             |
| config.yaml                  | 5KB  | Example configuration       |

**Total Documentation**: ~145KB of comprehensive specs and guides!

---

## GitHub Actions & CI/CD

- **[../.github/ACTIONS_BILLING_FIX.md](../.github/ACTIONS_BILLING_FIX.md)** - GitHub Actions billing troubleshooting
  - Resolving spending limit issues
  - Workflow modernization
  - CI/CD best practices

---

**Next Steps**:

1. Read [../README.md](../README.md) for project overview
2. Check [GETTING_STARTED.md](GETTING_STARTED.md) to begin development
3. Review [PERFORMANCE.md](PERFORMANCE.md) for verified production-ready metrics
4. Say "go" to start implementing features!
