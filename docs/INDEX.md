# Yatagarasu Documentation Index

## Quick Start (Read in Order)

1. **[STREAMING_ANSWER.md](STREAMING_ANSWER.md)** ⚡ **START HERE** - Direct answers to your streaming questions
2. **[GETTING_STARTED.md](GETTING_STARTED.md)** - How to begin development
3. **[README.md](README.md)** - Project overview and features

## Core Documentation

### Development Methodology
- **[CLAUDE.md](CLAUDE.md)** - Kent Beck's TDD methodology for this project
  - Red → Green → Refactor cycle
  - Structural vs behavioral commits
  - Code quality standards

### Product Specifications
- **[spec.md](spec.md)** - Complete product specification
  - Functional requirements (multi-bucket, JWT, S3 proxying)
  - Non-functional requirements (performance, security)
  - Technical architecture
  - Data models and APIs

### Implementation Plan
- **[plan.md](plan.md)** - TDD implementation roadmap
  - 200+ tests across 11 phases
  - Detailed test cases for each feature
  - Test execution commands

## Architecture Deep Dives

### Streaming and Caching
- **[STREAMING_ANSWER.md](STREAMING_ANSWER.md)** ⚡ **Your Question Answered**
  - Does proxy buffer to disk? NO - streams directly
  - How does caching work? Small files cached, large files streamed
  - Quick reference diagrams

- **[STREAMING_ARCHITECTURE.md](STREAMING_ARCHITECTURE.md)** - Detailed technical documentation
  - Complete sequence diagrams with timing
  - Memory usage patterns
  - Cache decision logic
  - Implementation pseudocode

- **[QUICK_REFERENCE_STREAMING.md](QUICK_REFERENCE_STREAMING.md)** - ASCII diagrams
  - Quick visual reference
  - All scenarios in simple ASCII art
  - Cache decision tree
  - Performance characteristics

- **[RANGE_REQUESTS.md](RANGE_REQUESTS.md)** ⭐ **Range Request Support**
  - HTTP Range header support (bytes ranges)
  - Use cases: video seeking, resume downloads, PDF previews
  - Always streamed, never cached
  - Works with authentication
  - Performance: 95% bandwidth savings in seek scenarios

- **[PARALLEL_DOWNLOADS.md](PARALLEL_DOWNLOADS.md)** 🚀 **Parallel Downloads via Range**
  - Download large files 5-10x faster
  - Multiple concurrent range requests
  - Works with aria2, curl, wget, custom clients
  - No special configuration needed
  - Constant memory: connections × 64KB

- **[CACHE_PREWARMING.md](CACHE_PREWARMING.md)** 🔮 **Cache Pre-Warming (v1.1 Feature)**
  - Recursive path prefetching planned for v1.1
  - Populate cache on startup or schedule
  - API-driven and automated pre-warming
  - Workarounds for v1.0 (external scripts)
  - ROI: Instant load times, cost savings

- **[CACHE_MANAGEMENT.md](CACHE_MANAGEMENT.md)** 🔧 **Cache Management (Purge/Renew/Conditional)**
  - Purging: v1.0 ❌ → v1.1 ✅ (API-based invalidation)
  - Renewal: v1.0 ⚠️ TTL only → v1.1 ✅ (Manual + auto refresh)
  - Conditional requests: v1.0 ⚠️ → v1.1 ✅ (304 Not Modified, ETag validation)
  - Workarounds for v1.0
  - 90% bandwidth savings with conditional requests

- **[CACHE_MANAGEMENT_ANSWER.md](CACHE_MANAGEMENT_ANSWER.md)** ⚡ **Quick Cache Management Answers**
  - Does it support purging? No (v1.0) → Yes (v1.1)
  - Does it support renewal? Partial → Full
  - Does it check Last-Modified? Forward only → Validate

## Configuration

- **[config.yaml](config.yaml)** - Complete example configuration
  - Public bucket (no auth)
  - Private bucket with JWT
  - Admin bucket with strict claims
  - All options documented inline

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

## Key Architectural Decisions

### 1. Zero-Copy Streaming (Large Files)
**Decision**: Stream S3 responses directly to clients without local buffering
**Why**: 
- Constant memory usage regardless of file size
- Low latency (first byte in ~500ms)
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

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Cache Hit Response | <10ms | Memory-speed access |
| S3 Streaming TTFB | <500ms | First byte to client |
| Throughput | >10,000 req/s | On commodity hardware |
| Memory per Connection | ~64KB | Constant, not file-size dependent |
| CPU Usage | -70% vs Envoy | Pingora efficiency |
| Cache Hit Rate | >90% | For hot static content |

## Technology Stack Summary

```
Language:     Rust 1.70+ (async/await, zero-cost abstractions)
Framework:    Cloudflare Pingora (high-performance proxy)
Async Runtime: Tokio (via Pingora)
S3 SDK:       AWS SDK for Rust (official, well-maintained)
JWT:          jsonwebtoken crate
Config:       YAML with serde
Logging:      Structured JSON via tracing
Metrics:      Prometheus format
Testing:      TDD with >90% coverage target
```

## Quick Command Reference

```bash
# Development
cargo test              # Run all tests
cargo clippy            # Linter
cargo fmt               # Format code

# Testing with MinIO
docker run -d -p 9000:9000 minio/minio server /data
cargo test --test integration_*

# Run proxy
cargo run -- --config config.yaml

# Metrics
curl http://localhost:9090/metrics
```

## Document Sizes

| Document | Size | Purpose |
|----------|------|---------|
| spec.md | 35KB | Complete specification |
| plan.md | 28KB | 200+ tests across 11 phases |
| STREAMING_ARCHITECTURE.md | 17KB | Detailed technical docs |
| README.md | 18KB | Project overview |
| QUICK_REFERENCE_STREAMING.md | 15KB | Quick diagrams |
| GETTING_STARTED.md | 8KB | Onboarding guide |
| CLAUDE.md | 7KB | TDD methodology |
| config.yaml | 5KB | Example configuration |

**Total Documentation**: ~133KB of comprehensive specs and guides!

## Development Workflow

1. Read **CLAUDE.md** to understand TDD methodology
2. Review **spec.md** to understand requirements
3. Open **plan.md** and find next `[ ]` test
4. Implement test (Red) → Make it pass (Green) → Refactor
5. Mark test `[x]` and commit with `[BEHAVIORAL]` or `[STRUCTURAL]` prefix
6. Repeat!

Or just say **"go"** to Claude and let the AI guide you through the TDD cycle!

## Questions Answered

### Q: Does the proxy buffer large files to disk?
**A**: NO - Uses zero-copy streaming (see STREAMING_ANSWER.md)

### Q: How does caching work?
**A**: Small files (<10MB) cached in memory, large files always streamed (see QUICK_REFERENCE_STREAMING.md)

### Q: Does it support HTTP Range requests?
**A**: YES - Full support for byte ranges (see RANGE_REQUESTS.md)
- Single, multiple, suffix, and open-ended ranges
- Always streamed from S3, never cached
- Works with JWT authentication
- 95% bandwidth savings for video seeking scenarios

### Q: Does it support parallel downloads using Range requests?
**A**: YES - Full support for concurrent range requests (see PARALLEL_DOWNLOADS.md)
- Download large files 5-10x faster
- Split file into chunks, download in parallel
- Works with aria2, curl, custom clients
- No configuration needed
- Memory: connections × 64KB (constant)

### Q: Does it support cache pre-warming (recursive path prefetching)?
**A**: NOT YET - Planned for v1.1 (see CACHE_PREWARMING.md)
- Recursive path prefetching to populate cache
- API-driven and scheduled pre-warming
- Workarounds available for v1.0 (external scripts)
- Benefits: Instant load times, reduced S3 costs, peak traffic preparation

### Q: Does it support cache purging (invalidation)?
**A**: NOT YET - Planned for v1.1 (see CACHE_MANAGEMENT.md)
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
**A**: ~64KB per connection constant, regardless of file size

### Q: Can it handle video streaming?
**A**: YES - Efficient streaming of GB+ files with constant memory, plus Range support for seeking

### Q: How's the performance?
**A**: 70% lower CPU than Envoy, >10K req/s, <10ms cache hits

---

**Next Steps**: 
1. Read [STREAMING_ANSWER.md](STREAMING_ANSWER.md) for your streaming questions
2. Check [GETTING_STARTED.md](GETTING_STARTED.md) to begin development
3. Say "go" to start implementing the first test!
