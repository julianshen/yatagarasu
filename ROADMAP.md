# Yatagarasu Implementation Roadmap

This document outlines the path from library foundation to production-ready S3 proxy.

---

## v0.1.0 - Library Foundation ✅ **COMPLETE**

**Status**: Released (Phases 1-11 complete)

**What's Included**:
- ✅ Configuration management (YAML + env vars)
- ✅ Multi-bucket routing with longest prefix matching
- ✅ JWT authentication and claims verification
- ✅ S3 client with AWS Signature v4
- ✅ HTTP Range request parsing
- ✅ Comprehensive test suite (373 tests, 98.43% coverage)

**Deliverables**:
- Library modules for config, router, auth, S3
- Full unit test coverage
- API documentation
- Example configurations

**What You Can Do**:
```rust
// Use library components in your own Rust application
let config = load_config("config.yaml")?;
let router = Router::new(config.buckets);
let claims = authenticate_request(&headers, &query, &jwt_config)?;
let s3_request = build_get_object_request(&bucket, &key, &region);
```

**Timeline**: Complete

---

## v0.2.0 - HTTP Server Integration 🚧 **IN PROGRESS**

**Status**: Phase 12+ (Current Sprint)

**Goal**: Transform library into working HTTP proxy server

### Phase 12: Pingora Server Setup

**Objective**: Initialize Pingora HTTP server and handle basic requests

**Tasks**:
- [ ] Add Pingora dependencies to Cargo.toml
- [ ] Create server configuration structure
- [ ] Initialize Pingora server in main.rs
- [ ] Configure listening address and port
- [ ] Implement basic HTTP request handler
- [ ] Add `/health` endpoint for health checks
- [ ] Test server starts and responds to HTTP requests

**Tests to Write** (~10 tests):
- Test: Server can bind to configured address
- Test: Server accepts HTTP connections
- Test: Server responds with 200 OK to /health
- Test: Server returns 404 for unknown paths
- Test: Server handles concurrent requests
- Test: Server can be stopped gracefully

**Expected Duration**: 2-3 days

### Phase 13: Request Pipeline Integration

**Objective**: Connect router, auth, and request context

**Tasks**:
- [ ] Create RequestContext struct (request ID, path, headers, query)
- [ ] Integrate Router to determine target bucket
- [ ] Add authentication middleware
- [ ] Pass context through pipeline
- [ ] Implement error responses (401, 403, 404)
- [ ] Add request logging

**Tests to Write** (~15 tests):
- Test: Router middleware extracts bucket from path
- Test: Auth middleware validates JWT when enabled
- Test: Auth middleware skips validation when disabled
- Test: Request ID is generated and tracked
- Test: 401 returned for missing JWT on private buckets
- Test: 403 returned for invalid JWT claims
- Test: 404 returned for unmapped paths
- Test: Public buckets accessible without JWT

**Expected Duration**: 3-4 days

### Phase 14: S3 Proxying Implementation

**Objective**: Fetch objects from S3 and stream to client

**Tasks**:
- [ ] Create S3 HTTP client (with connection pooling)
- [ ] Build S3 GET requests with signed headers
- [ ] Forward client Range headers to S3
- [ ] Stream S3 response to HTTP client
- [ ] Preserve S3 response headers (ETag, Content-Type, etc.)
- [ ] Handle S3 errors and map to HTTP status codes
- [ ] Implement HEAD request proxying

**Tests to Write** (~20 tests):
- Test: GET request proxies to S3 successfully
- Test: HEAD request proxies to S3 successfully
- Test: Response headers from S3 are preserved
- Test: Range requests are forwarded to S3
- Test: S3 404 returns HTTP 404
- Test: S3 403 returns HTTP 403
- Test: Large files stream without buffering (constant memory)
- Test: Client disconnect cancels S3 request
- Test: Concurrent requests to different buckets work
- Test: Each bucket uses correct credentials

**Expected Duration**: 4-5 days

### Phase 15: Error Handling & Logging

**Objective**: Production-ready error handling and observability

**Tasks**:
- [ ] Create centralized error module
- [ ] Implement user-friendly error responses
- [ ] Add structured logging with tracing
- [ ] Log all requests with context (request ID, bucket, user, duration)
- [ ] Log authentication failures with reasons
- [ ] Log S3 errors with details
- [ ] Ensure no sensitive data in logs (credentials, tokens)

**Tests to Write** (~10 tests):
- Test: Errors return JSON error responses
- Test: 500 errors don't leak implementation details
- Test: All requests are logged with request ID
- Test: Authentication failures are logged
- Test: S3 errors are logged with context
- Test: Credentials are never logged
- Test: JWT tokens are never logged

**Expected Duration**: 2-3 days

### Phase 16: Final Integration & Testing

**Objective**: End-to-end integration tests and polish

**Tasks**:
- [ ] Write integration tests with real MinIO
- [ ] Test multi-bucket routing end-to-end
- [ ] Test JWT authentication end-to-end
- [ ] Test Range request streaming
- [ ] Performance baseline tests
- [ ] Memory leak tests
- [ ] Load testing (concurrent requests)
- [ ] Update documentation with real examples

**Tests to Write** (~15 tests):
- Integration: Full request flow (public bucket)
- Integration: Full request flow (private bucket with JWT)
- Integration: Range requests work end-to-end
- Integration: Multiple buckets with different credentials
- Integration: Concurrent requests don't interfere
- Integration: Memory usage stays constant during streaming
- Performance: Throughput > 1,000 req/s (baseline)
- Performance: JWT validation < 1ms
- Performance: Path routing < 10μs

**Expected Duration**: 3-4 days

### v0.2.0 Release Criteria

**Must Have**:
- ✅ HTTP server accepts requests on configured port
- ✅ Routing directs requests to correct S3 bucket
- ✅ JWT authentication works for private buckets
- ✅ GET and HEAD requests proxy to S3
- ✅ Range requests stream correctly
- ✅ Errors return appropriate HTTP status codes
- ✅ All 373 existing tests still pass
- ✅ 50+ new integration tests passing
- ✅ Health check endpoint works
- ✅ Basic logging in place

**Nice to Have** (defer to v0.3.0 if needed):
- Connection pooling
- Request timeouts
- Retry logic

**Timeline**: 3-4 weeks from start of Phase 12

---

## v0.3.0 - Production Readiness 📋 **PLANNED**

**Goal**: Add observability, operations, and production hardening

### Phase 17: Prometheus Metrics

**Tasks**:
- [ ] Add prometheus crate dependency
- [ ] Create metrics registry
- [ ] Export metrics on `/metrics` endpoint (separate port)
- [ ] Add request counter (by bucket, status code, method)
- [ ] Add request duration histogram
- [ ] Add in-flight requests gauge
- [ ] Add S3 request metrics
- [ ] Add authentication metrics (success/failure rate)

**Expected Duration**: 2-3 days

### Phase 18: Configuration Hot Reload

**Tasks**:
- [ ] Watch config file for changes
- [ ] Implement SIGHUP signal handler
- [ ] Reload configuration without dropping connections
- [ ] Validate new config before applying
- [ ] Keep old config if validation fails
- [ ] Log reload attempts and results
- [ ] Test: Config reload without downtime
- [ ] Test: Invalid config doesn't break running server

**Expected Duration**: 3-4 days

### Phase 19: Graceful Shutdown

**Tasks**:
- [ ] Implement SIGTERM signal handler
- [ ] Stop accepting new connections
- [ ] Wait for in-flight requests to complete
- [ ] Implement shutdown timeout (e.g., 30 seconds)
- [ ] Close S3 connections gracefully
- [ ] Log shutdown events
- [ ] Test: No requests dropped during shutdown
- [ ] Test: Shutdown timeout works

**Expected Duration**: 2 days

### Phase 20: Observability & Production Hardening

**Tasks**:
- [ ] Add distributed tracing support
- [ ] Implement request/response logging
- [ ] Add connection pooling for S3
- [ ] Implement request timeouts
- [ ] Add retry logic with exponential backoff
- [ ] Implement circuit breaker for failing S3 endpoints
- [ ] Add rate limiting (optional feature flag)
- [ ] Performance tuning and optimization

**Expected Duration**: 5-7 days

### v0.3.0 Release Criteria

**Must Have**:
- ✅ Prometheus metrics endpoint
- ✅ Configuration hot reload (SIGHUP)
- ✅ Graceful shutdown (SIGTERM)
- ✅ Request timeouts
- ✅ Retry logic for transient S3 failures
- ✅ All integration tests pass
- ✅ Load testing shows stable performance
- ✅ Memory usage remains constant under load

**Timeline**: 2-3 weeks after v0.2.0

---

## v0.4.0 - Docker & CI/CD 🐳 **PLANNED**

**Goal**: Production deployment automation and continuous integration

### Phase 21: Docker Image Creation

**Tasks**:
- [ ] Create multi-stage Dockerfile for minimal image size
- [ ] Use rust:1.70-slim as builder, distroless/cc as runtime
- [ ] Copy only binary and necessary runtime dependencies
- [ ] Set up proper signal handling for container lifecycle
- [ ] Configure logging to stdout for container environments
- [ ] Add health check configuration in Dockerfile
- [ ] Build and tag image (yatagarasu:latest, yatagarasu:v0.4.0)
- [ ] Optimize image size (<50MB if possible)
- [ ] Test: Docker image builds successfully
- [ ] Test: Container starts with config volume mount
- [ ] Test: Container responds to health checks
- [ ] Test: Container handles SIGTERM gracefully
- [ ] Test: Container logs to stdout in JSON format
- [ ] Test: Environment variables override config values

**Expected Duration**: 2-3 days

### Phase 22: Docker Compose for Testing

**Tasks**:
- [ ] Create docker-compose.yml with yatagarasu + MinIO
- [ ] Configure MinIO service with test buckets
- [ ] Pre-populate MinIO with test data on startup
- [ ] Configure yatagarasu to connect to MinIO
- [ ] Add volume mounts for config and test data
- [ ] Add network configuration for service communication
- [ ] Create example .env file for credentials
- [ ] Add healthcheck configuration for both services
- [ ] Test: docker-compose up brings up both services
- [ ] Test: Can access MinIO console at localhost:9001
- [ ] Test: Can send requests to proxy at localhost:8080
- [ ] Test: Proxy successfully fetches from MinIO
- [ ] Test: docker-compose down cleans up properly

**Deliverables**:
- `Dockerfile` - Multi-stage production image
- `docker-compose.yml` - Full testing environment
- `docker-compose.test.yml` - Minimal test setup
- `.env.example` - Example environment variables
- `docs/DOCKER.md` - Docker deployment guide

**Expected Duration**: 2 days

### Phase 23: GitHub Actions CI/CD

**Tasks**:
- [ ] Create .github/workflows/ci.yml
- [ ] Add job: cargo test (run all tests)
- [ ] Add job: cargo clippy (lint checks)
- [ ] Add job: cargo fmt --check (format checks)
- [ ] Add job: cargo tarpaulin (coverage report)
- [ ] Add job: cargo build --release (ensure builds)
- [ ] Upload coverage reports to Codecov
- [ ] Add job: Docker image build
- [ ] Add job: Integration tests with MinIO (docker-compose)
- [ ] Add job: Security audit (cargo audit)
- [ ] Configure caching for Cargo dependencies
- [ ] Run CI on push to main and pull requests
- [ ] Test: CI pipeline runs successfully on push
- [ ] Test: Failed tests block merge
- [ ] Test: Clippy warnings block merge
- [ ] Test: Coverage threshold enforced (>90%)

**Expected Duration**: 2-3 days

### Phase 24: Container Registry & Release

**Tasks**:
- [ ] Create .github/workflows/release.yml
- [ ] Trigger on git tags (v*.*.*)
- [ ] Build Docker images for multiple platforms (amd64, arm64)
- [ ] Push images to GitHub Container Registry (ghcr.io)
- [ ] Tag images with version and latest
- [ ] Create GitHub Release with changelog
- [ ] Attach compiled binaries to release (Linux, macOS)
- [ ] Generate and attach SBOM (Software Bill of Materials)
- [ ] Add release notes template
- [ ] Test: Release workflow on tag push
- [ ] Test: Images available on ghcr.io
- [ ] Test: Binaries download and run

**Expected Duration**: 2 days

### v0.4.0 Release Criteria

**Must Have**:
- ✅ Dockerfile builds production image (<100MB)
- ✅ docker-compose.yml provides full test environment
- ✅ CI pipeline runs all tests and checks
- ✅ CI enforces code quality (tests, clippy, fmt)
- ✅ Docker images pushed to container registry
- ✅ Automated releases on git tags
- ✅ Docker deployment documentation

**Nice to Have**:
- Multi-architecture builds (amd64, arm64)
- Security scanning in CI (trivy, snyk)
- Automated dependency updates (dependabot)
- Performance regression testing in CI

**Timeline**: 1-2 weeks after v0.3.0

---

## v1.0.0 - First Stable Release 🎯 **FUTURE**

**Goal**: Production-ready S3 proxy with caching

### Phase 25: Memory Caching

**Tasks**:
- [ ] Implement LRU cache for small files (<10MB)
- [ ] Cache GET responses (not Range requests)
- [ ] Respect Cache-Control headers
- [ ] Implement cache size limits
- [ ] Add cache hit/miss metrics
- [ ] Test: Cache reduces S3 requests
- [ ] Test: Cache respects size limits
- [ ] Test: Stale cache entries are evicted

**Expected Duration**: 4-5 days

### Phase 26: Cache Management

**Tasks**:
- [ ] Add cache invalidation API
- [ ] Support conditional requests (If-None-Match, If-Modified-Since)
- [ ] Implement cache warming (optional)
- [ ] Add cache statistics endpoint
- [ ] Test: Cache invalidation works
- [ ] Test: Conditional requests save bandwidth

**Expected Duration**: 3-4 days

### Phase 27: Advanced Features

**Optional enhancements based on user feedback**:
- [ ] RS256/ES256 JWT algorithms
- [ ] Multi-region S3 failover
- [ ] Rate limiting per client
- [ ] Request/response transformation hooks
- [ ] WebSocket support for event streaming

**Expected Duration**: Variable, based on priorities

### v1.0.0 Release Criteria

**Must Have**:
- ✅ All v0.3.0 features stable
- ✅ Memory caching operational
- ✅ Cache invalidation API
- ✅ Comprehensive documentation
- ✅ Production deployment guide
- ✅ Performance benchmarks published
- ✅ Security audit complete
- ✅ At least 3 production deployments

**Timeline**: 3-6 months after v0.3.0

---

## Development Principles

Throughout all phases, we maintain:

### ✅ TDD Discipline
- **Red**: Write failing test first
- **Green**: Implement minimum code to pass
- **Refactor**: Improve structure while keeping tests green
- **Commit**: Separate [STRUCTURAL] and [BEHAVIORAL] commits

### ✅ Quality Standards
- All tests must pass before committing
- No clippy warnings
- Code properly formatted (cargo fmt)
- Test coverage >90%
- Clear, descriptive commit messages

### ✅ Architecture Principles
- Separation of concerns
- Explicit dependencies
- Minimal state and side effects
- Fail fast and loudly
- Security by default

---

## Success Metrics

### v0.2.0 (Working Proxy)
- Can proxy GET/HEAD requests to S3
- JWT authentication works
- Multi-bucket routing works
- Handles 1,000+ req/s
- Memory usage constant during streaming

### v0.3.0 (Production Ready)
- Prometheus metrics available
- Hot reload works without downtime
- Graceful shutdown completes cleanly
- Retry logic handles transient failures
- Runs stable for 72+ hours under load

### v1.0.0 (Feature Complete)
- Cache reduces S3 requests by >50%
- Handles 10,000+ req/s
- P95 latency <100ms (cached), <500ms (S3)
- Memory usage <500MB baseline
- Used in production by multiple teams

---

## Current Focus: v0.2.0 HTTP Server Integration

**We are here**: Starting Phase 12 - Pingora Server Setup

**Next "go" command will**:
1. Read plan.md Phase 12 section
2. Find first unmarked test for Pingora server
3. Write test (Red phase)
4. Implement minimal code (Green phase)
5. Refactor if needed
6. Mark test complete
7. Commit with appropriate prefix

**Estimated time to v0.2.0**: 3-4 weeks of focused development

**Ready?** Say **"go"** to start Phase 12! 🚀
